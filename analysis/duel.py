"""One focused comparison: learned vs utility on Choke, identical neutral teams, both sides."""
import argparse,concurrent.futures,csv,hashlib,json,statistics,subprocess
from pathlib import Path
from analysis.compare import interval
from analysis.experiments import ROOT,metrics,digest

def run(args):
    output=Path(args.output).resolve()
    if output.exists():raise ValueError("use a fresh output directory")
    output.mkdir(parents=True)
    subprocess.run(["cargo","build","--locked","--no-default-features","--bin","phalanx-batch"],cwd=ROOT,check=True)
    binary=ROOT/"target/debug/phalanx-batch"
    manifest={"schema_version":1,"map":"choke","seeds":list(range(args.start,args.start+args.seeds)),"orientations":[False,True],"traits":"all seven = 0.5 for both teams","roles":"3 melee + 2 rock throwers each","primary":"learner-minus-utility match score, side-averaged per seed","git_commit":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),"tracked_dirty":bool(subprocess.check_output(["git","status","--porcelain","--untracked-files=no"],cwd=ROOT)),"model_pt_sha256":digest(ROOT/"docs/results/v04/policy/policy.pt"),"export_sha256":digest(ROOT/"assets/policy.json"),"binary_sha256":digest(binary),"source_sha256":{str(p.relative_to(ROOT)):digest(p) for p in sorted((ROOT/"src").glob("*.rs"))}}
    def trial(item):
        seed,swap=item;stem=f"choke-{seed:04}-{'swap' if swap else 'normal'}";path=output/(stem+".json")
        cmd=[str(binary),"--arena","choke","--seed",str(seed),"--duel","--output",str(path)]
        if swap:cmd.append("--swap")
        subprocess.run(cmd,check=True,capture_output=True)
        b=json.loads(path.read_text());team=b["learned_team"]
        winner={"Red wins":0,"Blue wins":1,"Draw":None,"Timeout / draw":None}[b["result"]]
        score=.5 if winner is None else float(winner==team)
        units=[u for u in b["units"] if u["team"]==team];decisions=sum(sum(u["decision_counts"]) for u in units)
        disagreements=sum(r["unit"]//5==team for r in b["disagreements"])
        row={"seed":seed,"swapped":int(swap),"learner_score":score,"score_advantage":score*2-1,"duration":b["tick"]/10,"timeout":int(b["result"]=="Timeout / draw"),"learner_decisions":decisions,"learner_disagreements":disagreements,"learned_fallbacks":b["learned_fallbacks"],"summary":path.name}
        for label,side in [("learned",team),("utility",1-team)]:row.update({label+"_"+k:v for k,v in metrics(b,side).items()})
        return row
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(trial,[(s,side) for s in manifest["seeds"] for side in [False,True]]))
    with (output/"battles.csv").open("w",newline="") as f:
        w=csv.DictWriter(f,fieldnames=list(rows[0]));w.writeheader();w.writerows(rows)
    effects=[statistics.fmean(r["score_advantage"] for r in rows if r["seed"]==s) for s in manifest["seeds"]]
    wins=sum(r["learner_score"]==1 for r in rows);draws=sum(r["learner_score"]==.5 for r in rows)
    report={"seed_blocks":args.seeds,"battles":len(rows),"learned_wins":wins,"utility_wins":len(rows)-wins-draws,"draws":draws,"timeouts":sum(r["timeout"] for r in rows),"score_advantage":statistics.fmean(effects),"score_advantage_95ci":interval(effects),"learned_fallbacks":sum(r["learned_fallbacks"] for r in rows),"same_state_agreement_on_learned_trajectories":1-sum(r["learner_disagreements"] for r in rows)/sum(r["learner_decisions"] for r in rows)}
    (output/"report.json").write_text(json.dumps(report,indent=2)+"\n")
    manifest["files_sha256"]={p.name:digest(p) for p in sorted(output.iterdir())}
    (output/"manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
    print(json.dumps(report,indent=2))
if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("--output",required=True);p.add_argument("--start",type=int,default=100);p.add_argument("--seeds",type=int,default=32);a=p.parse_args()
    if a.seeds<2:p.error("at least two seed blocks required")
    run(a)
