use phalanx::{
    async_exchange::{ChallengeService, LocalExchange},
    challenges::TeamSnapshot,
    progression::Campaign,
};
#[test]
fn offline_owner_receives_one_verified_result_for_the_issued_frozen_publication() {
    let a = Campaign::new();
    let b = Campaign::new();
    let originals = [
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap(),
    ];
    let mut service = LocalExchange::default();
    let owner = service
        .publish("penelope", TeamSnapshot::from_campaign(&a, 0).unwrap())
        .unwrap();
    // Simulate a service restart while the owner is absent.
    let mut service: LocalExchange =
        serde_json::from_str(&serde_json::to_string(&service).unwrap()).unwrap();
    let challenger = service
        .publish("challenger", TeamSnapshot::from_campaign(&b, 0).unwrap())
        .unwrap();
    assert_eq!(service.discover(3).len(), 2);
    assert!(service.discover(4).is_empty());
    let ticket = service.issue(challenger.id, owner.id, 42).unwrap();
    let result = ticket.challenge.run().unwrap();
    let mut tampered = result.clone();
    tampered.battle["tick"] = serde_json::json!(1);
    assert!(service.submit(ticket.id, tampered).is_err());
    assert!(service.inbox("penelope").is_empty());
    let mut wrong = result.clone();
    wrong.challenge.seed = 43;
    assert!(service.submit(ticket.id, wrong).is_err());
    // Publishing a newer build does not replace the ticket's original frozen teams.
    let mut changed = a.clone();
    changed.teams[0].fighters[0].mind.caution = 0.15;
    service
        .publish(
            "penelope",
            TeamSnapshot::from_campaign(&changed, 0).unwrap(),
        )
        .unwrap();
    assert_eq!(service.discover(3).len(), 2);
    service.submit(ticket.id, result.clone()).unwrap();
    service.submit(ticket.id, result).unwrap();
    assert_eq!(service.inbox("penelope").len(), 1);
    assert_eq!(service.inbox("challenger").len(), 1);
    assert!(service.inbox("unrelated").is_empty());
    let restored: LocalExchange =
        serde_json::from_str(&serde_json::to_string(&service).unwrap()).unwrap();
    restored.inbox("penelope")[0].result.verify().unwrap();
    assert_eq!(
        originals,
        [
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        ]
    );
}
