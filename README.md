Roomba
======

Installation
------------

```
git clone https://github.com/cecton/roomba
cd roomba
cargo install --path roomba-cli
```

Usage
-----

In order for it to work you will need to find out a few information:

1. The IP address of the device
2. The user and password
3. Optional: if you want to be able to clean a room or a set of rooms, you will
   need the `pmap_id` and `user_pmapv_id`.

### Find the IP address

You can use the command:

```
roomba-s9plus-cli find-ip
```

This command will run indefinitely and show all the Roomba thingies on your
network.

### Find the user and password

```
roomba-s9plus-cli get-password
```

This command will wait until the Roomba is in pairing state. You need to hold
the home button for 2 seconds to get the led ring blinking blue.

### Clean specific rooms

#### Find the `pmap_id` and `user_pmapv_id` and `region_id`s

Before continuing, please run a clean on a specific room with the Roomba app.
Then:

```
roomba-s9plus-cli command ssl://<roomba_ip>:8883 <blid> <password>
```

**Note:** Use `find-ip` to get the blid and `get-password` to get the password.

The `pmap_id`, `user_pmapv_id` and `region_id`s should be somewhere in the
JSON.

> $aws/things/XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX/shadow/update: {"state":{"reported":{"batPct": 100, ..., "lastCommand": {"command": "start", "initiator": "localApp", "time": 1593182552, "pmap_id": "cWvsuVZwSOmTBN6FTFR95Q", "user_pmapv_id": "200618T164453", "ordered": 1, "regions": [{"region_id": "17", "type": "rid"}]}, "lastDisconnect": 0, ... "vacHigh": false}}}

### Run the command

```
roomba-s9plus-cli command ssl://<roomba_ip>:8883 \
    <blid> <password> start-regions --ordered <user_pmapv_id> <pmap_id> 17
# 17 is the ID of the room (region_id)
```
