# qBit Controller

A simple manager for qBitTorrent. It gives you more control over your torrents.

## Features
- Tag torrents based on name

## TODO
- [ ] Option to turn on Auto Torrent Management
- [ ] Move categories based on tags and source category
- [ ] Share limits
- [ ] Tracker management
- [ ] Blocklist management


## Local Testing

### Build
```sh
docker build -t qbit-controller .
```

### Run
```sh
docker run --rm -it`
    -v "${PWD}/docker_config:/config" `
    -e qbit_con_schedule=120 `
    qbit-controller
```
