# qbit_controller

A small Rust daemon that maintains qBittorrent state on a schedule. Inspired by
[qbit_manage](https://github.com/StuffAnThings/qbit_manage), with a narrower
feature set focused on tagging, share-limit management, and category routing.

## What it does

Each run pulls the current torrent list from qBittorrent and applies these
processes (each individually toggleable):

- **Auto-management** — flips qBittorrent's automatic torrent management on for
  every torrent except those carrying a configured ignore tag.
- **Tracker tags** — adds tags based on tracker URL substring match (e.g.
  `iptorrents` for any tracker URL containing `iptorrents`).
- **Tracker error tag** — applies a configurable tag (default `issue`) to
  torrents where at least one real (`http`/`https`/`udp`/`ws`/`wss`) tracker
  is in qBittorrent's *Not working* state and no real tracker is *Working*.
  Matches qbit_manage's semantics, so DHT/PeX/LSD pseudo-trackers and
  partially-working torrents are not flagged.
- **Share limits** — applies grouped ratio / seeding-time / upload-speed
  limits to torrents matched by category and tag rules. Supports
  `min_num_seeds` and `min_seeding_time` protection (limits are temporarily
  cleared and a `MinSeedsNotMet` / `MinSeedTimeNotReached` tag is added until
  the threshold is met). Optional `cleanup` flag deletes the torrent + data
  once the configured ratio or seeding time is reached.
- **Tag by name** — adds tags when the torrent name contains a configured
  substring (e.g. release group).
- **Category moves** — rewrites a torrent's category when its current category
  + tag set matches a rule.

All operations are **diff-aware**: if qBittorrent's reported state already
matches the desired state (auto-tmm flag, ratio/seeding-time/upload limits,
managed tags), the API call and the log line are skipped. Steady-state runs
should produce only the section headers and a summary block.

## Configuration

Configuration lives at `config/config.yml`. On first start a fully-commented
`example_config.yml` and JSON schema are written next to it; copy the example
and edit. Minimal config:

```yaml
qbit:
  url: http://10.0.0.10:8080
  username: username
  password: password

settings:
  dry_run: false
  enable_auto_management: true

processes:
  tracker_tags: true
  tracker_errors: true
  share_limits: true

trackers:
  iptorrents|stackoverflow.tech:
    tags: iptorrents
  other:
    tags: other

share_limits:
  IPT:
    priority: 2
    max_seeding_time: 30d
    min_num_seeds: 10
    include_all_tags: [iptorrents]
    cleanup: false

  default:
    priority: 999
    max_ratio: 1
    max_seeding_time: 1d
    categories: [movies-complete, tv-complete]
    cleanup: true
```

See `config/example_config.yml` for the full surface area.

### Notable settings

| Key                                       | Default                | Purpose                                                              |
|-------------------------------------------|------------------------|----------------------------------------------------------------------|
| `settings.dry_run`                        | `false`                | Log every action with a `[DRY-RUN]` prefix; never call write APIs.   |
| `settings.share_limits_tag`               | `z`                    | Prefix for managed group tags (`z_<priority>.<group>`).              |
| `settings.tracker_error_tag`              | `issue`                | Tag applied for tracker errors.                                      |
| `settings.share_limits_min_num_seeds_tag` | `MinSeedsNotMet`       | Protection tag while waiting for seeders.                            |
| `settings.share_limits_filter_completed`  | `true`                 | Skip incomplete torrents in the share-limits pass.                   |
| `settings.auto_management_ignore_tags`    | `[]`                   | Torrents with any of these tags are skipped by auto-management.      |

## Running

### Docker (recommended)

```sh
docker build -t qbit_controller .
docker run --rm -it \
  -v "$PWD/docker_config:/config" \
  -e qbit_con_schedule=120 \
  qbit_controller
```

`qbit_con_schedule` is the loop interval in **minutes** (default `120`). The
container runs the binary, sleeps for that many minutes, and repeats.

### Native

```sh
cargo run --release
```

The binary expects to be invoked from a directory containing `config/config.yml`
and `log_config.yml`. It runs once and exits.

## Logging

Logs are written to stdout and `config/qbit_controller.log` (rotated at 10MB,
5 archives). Log level and pattern live in `log_config.yml`.

Sample run with diff-aware output and section dividers:

```
=========== Auto management ===========
auto-tmm   12 torrent(s)
=========== Tracker tags ===========
tag-add    'Some Show S01E02' tags=["issue"]
=========== Share limits ===========
tag-remove 'Wonder Man S01E02 ...' tags=["MinSeedsNotMet"]
apply      'Wonder Man S01E02 ...' group=IPT
delete     '[SubsPlease] ... .mkv'  group=SubsPlease (limit reached)
=========== Category moves ===========
cat-move   'Some.Title.S01' anime-complete -> digital-core
==================== Summary ====================
  Torrents inspected         : 412
  Auto-management enabled    : 12
  Tracker tags added         : 1
  Tracker tags removed       : 0
  Share limits applied       : 8
  Share limits cleaned up    : 1
  Share-limit tags added     : 8
  Share-limit tags removed   : 8
  Name tags added            : 0
  Categories changed         : 1
```

In dry-run mode every action line is prefixed `[DRY-RUN]` and the summary
header reads `Summary (dry-run)`.

## Development

```sh
cargo test
cargo clippy --all-targets
cargo build --release
```
