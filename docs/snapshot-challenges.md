# Local snapshot challenges

Open **Snapshot challenges / exchange teams** from the main menu. Export your Phalanx, send the JSON file to another player, and challenge their published team. Both installations need the same game build. Leagues three and four are supported; both teams must have the same number of fighters.

## In the game

1. Prepare your fighters in the ordinary squad workshop. Finish any battle already in progress.
2. Open **Snapshot challenges** and click **Export my Phalanx**. The notice gives the new file's location under `outputs/challenges/outbox`.
3. Exchange team files. Drop an opponent's JSON file onto the snapshot page, or copy it into `outputs/challenges/inbox`, click **Refresh inbox**, and select its filename. The inbox shows the first twelve JSON files in filename order; drag-and-drop accepts a file from any location. You can also launch with `phalanx --snapshot /path/to/team.json`.
4. Review the imported team and challenge seed, then click **Fight imported Phalanx**. The battle starts paused in a pressure arena generated from both frozen teams. Use the ordinary Run, Pause, Step, camera and inspection controls. **Reset** restarts those same teams with that same seed.
5. The completed challenge opens its report on the snapshot page. Click **Export battle result** and send the resulting JSON file back. The recipient can drop that result onto their snapshot page to replay and verify it.
6. **Return to my campaign** restores your original staged or completed match, squads, rivals, resources and previous report. Export the result before leaving if you want to keep it.

These are **exhibition matches**. No gold, salvage, XP, weapon practice, injuries or psychological consequences are applied to either owner's campaign. Equipment, skills, minds and the controller are frozen during the match; use the workshop before starting another challenge. Exiting the game during a challenge leaves the ordinary campaign save intact. Imported results never authorize rewards or mutations to the owner's live team.

## Headless exchange

Build once; the tool needs no graphics dependencies:

```bash
cargo build --locked --no-default-features --bin phalanx-challenge
```

Export a saved team on each installation (`--team 1` exports the local opponent instead):

```bash
target/debug/phalanx-challenge export \
  --save outputs/phalanx-save.json --output my-team.json
```

After receiving an opponent's file:

```bash
target/debug/phalanx-challenge fight \
  --home my-team.json --away opponent.json --seed 42 --output result.json
```

Return the result to the other installation and verify it:

```bash
target/debug/phalanx-challenge verify --result result.json
```

Exports require a new filename and cannot overwrite a campaign save or previous packet. Invalid schemas, mismatched game builds, malformed rosters, league mismatches, oversized files and results differing from replay are rejected. The headless tool never saves a campaign.

## What verification establishes

The team packet contains the published fighter identities, names, appearance, titles, class, minds, equipment, selected skills, mastery, experience, wounds, psychology, heraldry, resources and recent history. Its source battle sequence lets a different installation validate that history independently of its own campaign IDs. Combat IDs are namespaced by side, allowing even two published copies of the same team to fight; source IDs remain intact in the embedded snapshots.

Publishing resolves default appearance choices into explicit saved looks, including the body base, so importing a team on the opposite side preserves its appearance. The owner's campaign is not changed by that normalization.

The result embeds both frozen team packets, the seed, the generated initial arena and the complete serialized final battle. Verification reruns the authoritative simulation and compares the complete recorded battle. Utility control is fixed for this first exchange version; the bundled policy still supplies the existing shadow comparisons. JSON floating-point parsing preserves round trips for exact replay checks.

Compatibility uses a source fingerprint covering the Rust sources, build script, manifest, lockfile and bundled policy. It is a build identifier, not a cryptographic signature. Replay consistency does not prove a team's ownership or that its statistics were earned legitimately. Online identity, trusted rewards and result delivery belong to Phase M. Exact cross-platform floating-point equivalence remains unproven; this slice is verified across separate directories and processes on the same platform and build.

## Asynchronous service emulator

```bash
target/debug/phalanx-challenge emulate \
  --home my-team.json --away opponent.json --seed 42 --output async-result.json
target/debug/phalanx-challenge verify --result async-result.json
```

The opponent publishes a frozen team. The local service is serialized and restored while its owner is absent. A challenger then publishes, receives a ticket binding both exact publications and the seed, fights, and submits a result. Replay verification precedes delivery to both owner inboxes. Resubmitting the identical result is idempotent; changed teams, seeds or recorded combat are rejected. Newer publications do not rewrite an issued match.

`src/async_exchange.rs` defines the `ChallengeService` boundary for publish, discovery, ticket issuance, submission and inbox retrieval. `LocalExchange` is its serializable, in-process implementation. A future network adapter can use this boundary without coupling combat or the UI to HTTP. The CLI demonstrates the whole local lifecycle; it does not start a server or persist a network mailbox. Local owner labels are fixtures, not authenticated accounts. A production implementation still needs identity, access control, trusted progression, storage validation, limits and hosted delivery. Exact replay establishes consistency, not legitimate ownership or earned statistics. Campaign rewards remain untouched.

The intended release keeps most play offline against NPCs, then reveals the asynchronous arena at the end of the larger campaign. That campaign gate and the online service remain future work. The five tutorial teams are the opening chapter, not the arena-unlock condition.

`tests/async_exchange.rs` verifies owner absence, service restoration, frozen tickets, duplicate submission, tampering rejection, both inboxes and unchanged source campaigns.

## Validation

[Rendered import screen](images/snapshot-import-witness.png) · [Completed challenge and result export](images/snapshot-result-witness.png). These captures use staged QA teams. The result exported by the rendered game was successfully verified by the headless tool, and the real campaign save retained its original checksum.

`tests/challenges.rs` exercises independent save histories, both playable leagues, full disk exchange and replay, unchanged source campaigns, modified-result rejection, validation limits, overlapping source identities, output overwrite protection and the actual command-line tool in separate installation directories. Game tests cover import through the button dispatcher, frozen controls and workshop actions, reset, completion, exact replay and restoration of the original campaign and match.

Rendered QA can run from a disposable working directory with an `assets` symlink:

```bash
/path/to/phalanx --visual-witness --snapshot-witness --seed 42
/path/to/phalanx --visual-witness --snapshot-witness --snapshot-report-witness --seed 42
```

Use a fresh directory per run. These stage test teams and exercise the ordinary import, start and export controls; they are QA fixtures rather than player progression. Captures appear under that directory's `outputs`.
