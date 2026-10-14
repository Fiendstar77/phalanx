"""Standalone experiment and learned-policy figures from actual recorded metrics."""
import argparse,json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

def plot_policy(directory):
    path=Path(directory);m=json.loads((path/"metrics.json").read_text())
    fig,axes=plt.subplots(1,2,figsize=(13,4.8),layout="constrained")
    history=m["history"]
    axes[0].plot([r["epoch"] for r in history],[r["validation_accuracy"] for r in history],label="Accuracy")
    axes[0].plot([r["epoch"] for r in history],[r["validation_macro_f1"] for r in history],label="Macro-F1")
    axes[0].set(xlabel="Epoch",ylabel="Validation score",ylim=(0,1),title="Selection uses validation seeds only");axes[0].legend()
    confusion=m["test"]["confusion_matrix"]
    fractions=[[value/max(1,sum(row)) for value in row] for row in confusion]
    image=axes[1].imshow(fractions,cmap="Blues",vmin=0,vmax=1)
    fig.colorbar(image,ax=axes[1],label="Fraction of teacher class")
    labels=[a.replace(" ","\n") for a in m["actions"]]
    axes[1].set(xticks=range(len(labels)),yticks=range(len(labels)),xticklabels=labels,yticklabels=labels,xlabel="Predicted action",ylabel="Utility teacher action",title="Held-out test · row-normalized confusion")
    axes[1].tick_params(labelsize=7)
    axes[1].set_xticklabels(labels,rotation=40,ha="right",rotation_mode="anchor")
    fig.suptitle("PHALANX · PyTorch policy imitation · combat improvement not established")
    fig.savefig(path/"learning.png",dpi=170);plt.close(fig)

def plot_study(report_file,output):
    data=json.loads(Path(report_file).read_text());names=list(data)
    effect=[data[n]["score_advantage"] for n in names]
    ci=[data[n]["score_advantage_95ci"] for n in names]
    fig,ax=plt.subplots(figsize=(7,4.5),layout="constrained")
    ax.errorbar(effect,range(len(names)),xerr=[[e-c[0] for e,c in zip(effect,ci)],[c[1]-e for e,c in zip(effect,ci)]],fmt="o",capsize=5)
    ax.axvline(0,color="gray",ls="--");ax.set(yticks=range(len(names)),yticklabels=names,xlim=(-1.05,1.05),xlabel="High − low match score (95% seed-block bootstrap CI)",title="PHALANX · exploratory trait contrast")
    fig.savefig(output,dpi=170);plt.close(fig)
def plot_duel(directory, output):
    import csv, statistics
    path=Path(directory);report=json.loads((path/"report.json").read_text())
    with (path/"battles.csv").open() as stream:rows=list(csv.DictReader(stream))
    seeds=sorted({int(r["seed"]) for r in rows})
    effects=[statistics.fmean(float(r["score_advantage"]) for r in rows if int(r["seed"])==seed) for seed in seeds]
    fig,axes=plt.subplots(1,2,figsize=(11,4.6),layout="constrained")
    bars=axes[0].bar(["Learned wins","Utility wins","Draws"],[report["learned_wins"],report["utility_wins"],report["draws"]],color=["#258c9e","#637189","#c5a657"])
    axes[0].bar_label(bars);axes[0].set(ylabel="Battles",ylim=(0,45),title="64 battles · identical teams · both spawn sides")
    axes[1].scatter(effects,[1 + ((i * 7) % 13 - 6) * .035 for i in range(len(effects))],alpha=.65,color="#637189",s=35)
    mean=report["score_advantage"];lo,hi=report["score_advantage_95ci"]
    axes[1].errorbar([mean],[0],xerr=[[mean-lo],[hi-mean]],fmt="o",capsize=6,color="#258c9e",markersize=9)
    axes[1].axvline(0,color="#637189",ls="--")
    axes[1].set(xlim=(-1.1,1.1),ylim=(-.5,1.5),yticks=[0,1],yticklabels=["Mean + 95% CI","Side-averaged seeds"],xlabel="Learned − utility match score",title=f"Advantage {mean:+.3f} · CI [{lo:+.3f}, {hi:+.3f}]")
    fig.suptitle("PHALANX · frozen PyTorch policy vs utility · Choke · fresh seeds 100–131")
    fig.savefig(output,dpi=170);plt.close(fig)

if __name__=="__main__":
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("--duel");p.add_argument("--policy");p.add_argument("--study");p.add_argument("--output");a=p.parse_args()
    if a.duel and a.output:plot_duel(a.duel,a.output)
    elif a.policy:plot_policy(a.policy)
    elif a.study and a.output:plot_study(a.study,a.output)
    else:p.error("use --policy DIRECTORY or --study REPORT --output PNG")
