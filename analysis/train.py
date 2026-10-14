"""Train a masked imitation policy; split whole seeds before fitting any preprocessing."""
import argparse, hashlib, json, random, time
from pathlib import Path
import torch
from torch import nn

class Policy(nn.Module):
    def __init__(self, inputs, actions):
        super().__init__()
        self.network = nn.Sequential(nn.Linear(inputs, 64), nn.ReLU(), nn.Linear(64, 64), nn.ReLU(), nn.Linear(64, actions))
    def forward(self, x):
        return self.network(x)

def load(paths, train_end=32, val_end=40):
    groups = {"train": [], "validation": [], "test": []}
    schemas = None
    source_schema = None
    for directory in paths:
        manifest=json.loads((Path(directory)/"manifest.json").read_text())
        if source_schema is not None and manifest["source_sha256"]!=source_schema: raise ValueError("mixed simulator source versions")
        source_schema=manifest["source_sha256"]
        for file in sorted(Path(directory).glob("*.jsonl")):
            with file.open() as stream:
                header = json.loads(next(stream))
                schema = (header["feature_names"], header["actions"])
                if schemas is not None and schema != schemas: raise ValueError("inconsistent feature/action schema")
                schemas = schema
                seed = header["seed"]
                split = "train" if seed < train_end else "validation" if seed < val_end else "test"
                for line in stream:
                    row = json.loads(line)
                    if row["event"] != "decision": continue
                    label = header["actions"].index(row["action"])
                    if not row["action_mask"][label]: raise ValueError("teacher selected an excluded action")
                    if len(row["features"]) != len(header["feature_names"]): raise ValueError("feature width mismatch")
                    groups[split].append((row["features"], row["action_mask"], label, seed))
    if schemas is None or any(not rows for rows in groups.values()): raise ValueError("nonempty train/validation/test seed groups required")
    tensors = {}
    for split, rows in groups.items():
        x, masks, labels, seeds = zip(*rows)
        tensors[split] = (torch.tensor(x, dtype=torch.float32), torch.tensor(masks, dtype=torch.bool), torch.tensor(labels), torch.tensor(seeds))
        if not torch.isfinite(tensors[split][0]).all(): raise ValueError("non-finite feature")
    seed_sets = [set(tensors[k][3].tolist()) for k in tensors]
    if any(a & b for i, a in enumerate(seed_sets) for b in seed_sets[i+1:]): raise ValueError("seed leakage")
    return tensors, schemas

def classification(pred, labels, actions):
    confusion = torch.bincount(labels * len(actions) + pred, minlength=len(actions)**2).reshape(len(actions), len(actions))
    classes = []
    for i, name in enumerate(actions):
        tp = int(confusion[i, i]); support = int(confusion[i].sum()); predicted = int(confusion[:, i].sum())
        precision = tp / predicted if predicted else 0.; recall = tp / support if support else None
        f1 = 2 * tp / (support + predicted) if support else None
        classes.append({"action": name, "support": support, "precision": precision, "recall": recall, "f1": f1})
    supported = [r["f1"] for r in classes if r["support"]]
    return {"accuracy": float((pred == labels).float().mean()), "macro_f1_supported": sum(supported) / len(supported), "classes": classes, "confusion_matrix": confusion.tolist()}

@torch.inference_mode()
def evaluate(model, x, mask, labels, actions):
    logits = model(x); raw = logits.argmax(1)
    result = classification(logits.masked_fill(~mask, -1e9).argmax(1), labels, actions)
    result["raw_eligible_action_rate"] = float(mask.gather(1, raw[:, None]).float().mean())
    result["masked_eligible_action_rate"] = 1.0
    return result

def run(args):
    torch.set_num_threads(4); torch.manual_seed(args.seed); random.seed(args.seed)
    torch.use_deterministic_algorithms(True)
    data, (features, actions) = load(args.data, args.train_end, args.val_end)
    output = Path(args.output)
    if output.exists(): raise ValueError("use a fresh output directory")
    output.mkdir(parents=True)
    mean = data["train"][0].mean(0); std = data["train"][0].std(0).clamp_min(1e-4)
    normalized = {name: ((x - mean) / std, masks, labels, seeds) for name, (x, masks, labels, seeds) in data.items()}
    model = Policy(len(features), len(actions)); optimizer = torch.optim.AdamW(model.parameters(), lr=1e-3)
    labels = data["train"][2]; counts = torch.bincount(labels, minlength=len(actions)).float()
    # Class weighting gives rare tactical choices a voice; absent classes get no training weight.
    weights = counts.sum() / counts.clamp_min(1).sqrt(); weights /= weights.mean()
    criterion = nn.CrossEntropyLoss(weight=weights)
    x, mask, y, _ = normalized["train"]
    history = []; best = -1.; start = time.perf_counter()
    checkpoint = output / "policy.pt"
    for epoch in range(args.epochs):
        model.train(); order = torch.randperm(len(y)); total = 0.
        for batch in order.split(1024):
            optimizer.zero_grad(); logits = model(x[batch]).masked_fill(~mask[batch], -1e9)
            loss = criterion(logits, y[batch]); loss.backward(); optimizer.step(); total += loss.item() * len(batch)
        model.eval(); validation = evaluate(model, *normalized["validation"][:3], actions)
        history.append({"epoch": epoch + 1, "loss": total / len(y), "validation_accuracy": validation["accuracy"], "validation_macro_f1": validation["macro_f1_supported"]})
        if validation["macro_f1_supported"] > best:
            best = validation["macro_f1_supported"]
            torch.save({"state_dict": model.state_dict(), "mean": mean, "std": std, "features": features, "actions": actions, "epoch": epoch + 1}, checkpoint)
        print(f"epoch {epoch+1}: loss {total/len(y):.4f}, val accuracy {validation['accuracy']:.3f}, macro-F1 {validation['macro_f1_supported']:.3f}", flush=True)
    saved = torch.load(checkpoint, weights_only=True); reloaded = Policy(len(features), len(actions)); reloaded.load_state_dict(saved["state_dict"]); reloaded.eval()
    report = {"task": "utility-policy imitation, not generative AI or combat-outcome optimization", "torch": torch.__version__, "seed": args.seed,
              "feature_names": features, "actions": actions, "parameters": sum(p.numel() for p in model.parameters()), "best_epoch": saved["epoch"],
              "selection": "validation macro-F1 over classes present in validation", "epochs": args.epochs, "seconds": time.perf_counter()-start,
              "splits": {k: {"decisions": len(v[2]), "seeds": sorted(set(v[3].tolist()))} for k,v in data.items()},
              "dataset_manifests": {str(Path(p)/"manifest.json"): hashlib.sha256((Path(p)/"manifest.json").read_bytes()).hexdigest() for p in args.data},
              "test": evaluate(reloaded, *normalized["test"][:3], actions), "history": history}
    tx, tm, ty, _ = normalized["test"]
    majority = int(counts.argmax()); majority_pred = torch.where(tm[:, majority], torch.full_like(ty, majority), tm.float().argmax(1))
    attack = actions.index("ATTACK"); advance = actions.index("ADVANCE")
    simple = torch.where(tm[:, attack], torch.full_like(ty, attack), torch.where(tm[:, advance], torch.full_like(ty, advance), tm.float().argmax(1)))
    timings=[]
    with torch.inference_mode():
        for i in range(1010):
            sample=tx[i%len(tx):i%len(tx)+1];eligible=tm[i%len(tx):i%len(tx)+1]
            before=time.perf_counter_ns();reloaded(sample).masked_fill(~eligible,-1e9).argmax(1)
            if i>=10:timings.append((time.perf_counter_ns()-before)/1000)
    timings.sort();report["cpu_inference_us"]={"median":timings[len(timings)//2],"p95":timings[len(timings)*95//100],"samples":len(timings),"scope":"normalized single-observation forward pass plus mask/argmax; excludes loading and feature extraction"}
    report["baselines"] = {"train_majority_with_first_eligible_fallback": classification(majority_pred,ty,actions), "attack_else_advance_else_first_eligible": classification(simple,ty,actions), "uniform_eligible_expected_accuracy": float((1./tm.sum(1)).mean())}
    # Save reload witness so inference can be checked independently of training.
    witness_x = data["test"][0][:32]; witness_mask = data["test"][1][:32]
    witness = {"features": witness_x.tolist(), "masks": witness_mask.tolist(), "logits": reloaded((witness_x-mean)/std).tolist()}
    (output/"reload_witness.json").write_text(json.dumps(witness)+"\n")
    (output/"metrics.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({"test_accuracy": report["test"]["accuracy"], "test_macro_f1": report["test"]["macro_f1_supported"], "parameters":report["parameters"]}))

if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__); p.add_argument("--data", nargs="+", required=True); p.add_argument("--output", required=True)
    p.add_argument("--epochs",type=int,default=20); p.add_argument("--seed",type=int,default=2026)
    p.add_argument("--train-end",type=int,default=32); p.add_argument("--val-end",type=int,default=40)
    args = p.parse_args()
    if args.epochs<1 or args.train_end<1 or args.val_end<=args.train_end: p.error("positive epochs and 0 < train-end < val-end required")
    run(args)
