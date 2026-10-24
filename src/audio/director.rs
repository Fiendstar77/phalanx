//! Presentation-only score direction from actual battle state.
use phalanx::{Battle,distance};
pub const LAYERS:usize=6;
pub const NAMES:[&str;LAYERS]=["mischief","approach","clash","distress","victory","defeat"];
pub fn target(b:&Battle,menu:bool)->[f32;LAYERS] {
    if b.finished {return if b.winner()==Some(0) {[0.05,0.,0.,0.,0.85,0.]}else{[0.05,0.,0.,0.,0.,0.8]};}
    if menu || b.tick==0 {return [0.65,0.,0.,0.,0.,0.];}
    let mut nearest=f32::MAX;let mut engaged=0.;let mut pressure=0.;let mut alive=0.;
    for u in &b.units {
        if u.team==0 {
            if u.health<=0. {pressure+=1.;}else{pressure+=(1.-u.health_fraction()).powi(2);alive+=1.;}
        }
        if u.health<=0. {continue;}
        let close=b.units.iter().filter(|v|v.team!=u.team && v.health>0.).map(|v|distance(u.position,v.position)).fold(f32::MAX,f32::min);
        nearest=nearest.min(close);if close<=u.attack_range()+1. {engaged+=1.;}
    }
    let clash=(engaged/b.units.len() as f32).clamp(0.,1.);
    let tension=((18.-nearest)/14.).clamp(0.,1.);
    let distress=(pressure/b.team_size() as f32+(1.-alive/b.team_size() as f32)*0.4).clamp(0.,1.);
    [0.48-clash*0.25,tension*0.5,clash*0.9,distress*0.7,0.,0.]
}
pub fn fade(current:&mut [f32;LAYERS],wanted:[f32;LAYERS],dt:f32) {
    let alpha=1.-(-dt.max(0.)*2.4).exp();for (v,t) in current.iter_mut().zip(wanted) {*v+=(t-*v)*alpha;}
}
#[cfg(test)]mod tests {
    use super::*;use phalanx::terrain::Arena;
    #[test]fn score_responds_to_clashes_danger_and_results_without_changing_battle() {
        let mut b=Battle::with_founders(42,false,Arena::OpenField);let calm=target(&b,false);
        b.tick=10;for u in &mut b.units {u.position=[if u.team==0 {-0.5}else{0.5},0.];}
        let clash=target(&b,false);assert!(clash[2]>calm[2]);
        b.units[0].health=1.;let danger=target(&b,false);assert!(danger[3]>clash[3]);
        let before=serde_json::to_string(&b).unwrap();target(&b,false);assert_eq!(before,serde_json::to_string(&b).unwrap());
        b.finished=true;for u in &mut b.units {if u.team==1 {u.health=0.;}}
        assert!(target(&b,false)[4]>0.);assert_eq!(target(&b,false)[5],0.);
        let mut mix=calm;fade(&mut mix,clash,0.);assert_eq!(mix,calm);fade(&mut mix,clash,0.1);assert!(mix[2]>0. && mix[2]<clash[2]);
    }
}
