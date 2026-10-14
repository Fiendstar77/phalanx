"""Load trained weights and verify saved inference observations."""
import argparse,json
from pathlib import Path
import torch
from analysis.train import Policy

def verify(directory):
    saved=torch.load(Path(directory)/"policy.pt",weights_only=True)
    model=Policy(len(saved["features"]),len(saved["actions"]));model.load_state_dict(saved["state_dict"]);model.eval()
    witness=json.loads((Path(directory)/"reload_witness.json").read_text())
    x=torch.tensor(witness["features"]); mask=torch.tensor(witness["masks"],dtype=torch.bool)
    with torch.inference_mode(): logits=model((x-saved["mean"])/saved["std"])
    torch.testing.assert_close(logits,torch.tensor(witness["logits"]),atol=1e-5,rtol=1e-5)
    prediction=logits.masked_fill(~mask,-1e9).argmax(1)
    if not mask.gather(1,prediction[:,None]).all(): raise AssertionError("ineligible prediction")
    return {"reload_verified":True,"observations":len(x),"actions":[saved["actions"][i] for i in prediction.tolist()]}
if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("directory");a=p.parse_args();print(json.dumps(verify(a.directory),indent=2))
