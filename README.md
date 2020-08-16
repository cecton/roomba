Roomba
======

Known supported devices
-----------------------

 *  Roomba S9+
 *  Roomba 966

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

After this, the IP address and the user name of the first thingy will be saved
in `roomba.toml` in your configuration directory (usually `~/.config`).

### Find the user and password

```
roomba-s9plus-cli get-password
```

This command will wait until the Roomba is in pairing state. You need to hold
the home button for 2 seconds to get the led ring blinking blue. The password
will then be saved to the configuration file.

### Clean specific rooms

#### Find the `pmap_id` and `user_pmapv_id` and `region_id`s

```
roomba-s9plus-cli command
```

**Note:** Before continuing, use `find-ip` to get the blid and username. Use
`get-password` to get the password.

The `pmap_id`, `user_pmapv_id` should be displayed in the events.

The `region_id`s can be guessed by looking at the last command. Run the Roomba
once with the app and select all the rooms to discover all the ids.

##### Example

In `roomba.toml`:

```toml
hostname = "x.x.x.x"
username = "A41547F457A924C392B7923749823432"
password = ":1:1598392010:7j28GmnS59cJTmPn"
user_pmapv_id = "200618T999999"
pmap_id = "jkd93MkfLd83kDi893kfgQ"

[[rooms]]
name = "Dinning Room"
region_id = "1"
type = "rid"

[[rooms]]
name = "Bedroom"
region_id = "2"
type = "rid"

[[rooms]]
name = "Living Room"
region_id = "3"
type = "rid"

[[rooms]]
name = "Storage Room"
region_id = "4"
type = "rid"

[[rooms]]
name = "Entryway"
region_id = "5"
type = "rid"
```

### Run the terminal user interface

```
roomba-s9plus-cli command
```
