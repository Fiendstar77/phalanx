"""Export the trained PyTorch MLP for a small equivalent CPU forward pass in Rust."""
import argparse,hashlib,json
from pathlib import Path
import torch

def export(checkpoint,output):
    saved=torch.load(checkpoint,weights_only=True)
    keys=sorted((k for k in saved["state_dict"] if k.endswith(".weight")), key=lambda name: int(name.split(".")[1]))
    layers=[]
    for key in keys:
        index=key.split(".")[1]
        layers.append({"weight":saved["state_dict"][key].tolist(),"bias":saved["state_dict"][f"network.{index}.bias"].tolist()})
    schema=1 if saved.get("hidden",64)==64 and saved.get("depth",2)==2 else 2
    data={"schema_version":schema,"source_sha256":hashlib.sha256(Path(checkpoint).read_bytes()).hexdigest(),"features":saved["features"],"actions":saved["actions"],"mean":saved["mean"].tolist(),"std":saved["std"].tolist(),"layers":layers}
    Path(output).write_text(json.dumps(data,separators=(",",":"))+"\n")
if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("checkpoint");p.add_argument("output");a=p.parse_args();export(a.checkpoint,a.output)
