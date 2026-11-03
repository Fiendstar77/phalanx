"""Export a standalone figure for the held-out cognition forecast result."""
import argparse
import json
from pathlib import Path


def main():
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('report',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    report=json.loads(args.report.read_text())
    test=report['metrics']['test']
    fig,axes=plt.subplots(1,3,figsize=(11,4.5),layout='constrained')
    colors=['#597b8c','#c27c35','#688b50']
    panels=[('Next-action probability loss', ['Markov','PyTorch'],
             [test['markov']['negative_log_likelihood'],test['neural']['negative_log_likelihood']],
             'Negative log likelihood; lower is better',None),
            ('When the action changes', ['Markov','PyTorch'],
             [100*test['markov']['transition_accuracy'],100*test['neural']['transition_accuracy']],
             'Correct transitions (%)',(0,100)),
            ('All next decisions', ['Markov','PyTorch','Repeat last'],
             [100*test['markov']['accuracy'],100*test['neural']['accuracy'],100*test['persistence_accuracy']],
             'Correct decisions (%)',(0,100))]
    for ax,(title,names,values,label,limits) in zip(axes,panels):
        bars=ax.bar(names,values,color=colors[:len(names)],width=.65)
        ax.set_title(title,fontsize=11)
        ax.set_ylabel(label,fontsize=9)
        ax.spines[['top','right']].set_visible(False)
        if limits:ax.set_ylim(*limits)
        else:ax.set_ylim(0,max(values)*1.3)
        ax.bar_label(bars,labels=[f'{v:.2f}' if limits else f'{v:.3f}' for v in values],padding=4)
    fig.suptitle('PHALANX: causal decision-sequence forecast\nResearch model, held-out self-play encounters',fontsize=14)
    fig.supxlabel(f"Test seeds {report['provenance']['seeds']['test']}; {test['samples']:,} correlated decisions; "
                  f"{test['neural']['transition_count']} transitions. No combat-strength claim.",fontsize=9)
    args.output.mkdir(parents=True,exist_ok=True)
    fig.savefig(args.output/'forecast.png',dpi=180)
    fig.savefig(args.output/'forecast.pdf')
    plt.close(fig)


if __name__ == '__main__':main()
