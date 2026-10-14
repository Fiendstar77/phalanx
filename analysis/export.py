"""Export the trained PyTorch MLP for a small equivalent CPU forward pass in Rust."""
import argparse,hashlib,json
from pathlib import Path
import torch

def export(checkpoint,output):
    saved=torch.load(checkpoint,weights_only=True)
    layers=[]
    for i in [0,2,4]:
        layers.append({"weight":saved["state_dict"][f"network.{i}.weight"].tolist(),"bias":saved["state_dict"][f"network.{i}.bias"].tolist()})
    data={"schema_version":1,"source_sha256":hashlib.sha256(Path(checkpoint).read_bytes()).hexdigest(),"features":saved["features"],"actions":saved["actions"],"mean":saved["mean"].tolist(),"std":saved["std"].tolist(),"layers":layers}
    Path(output).write_text(json.dumps(data,separators=(",",":"))+"\n")
if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("checkpoint");p.add_argument("output");a=p.parse_args();export(a.checkpoint,a.output)
