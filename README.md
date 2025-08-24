# HARVEST code

A place to put HARVEST code that has not yet been migrated into its own
repository.






# Docker Container Notes

Gilbert created a Dockerfile on August 23 so that he can run code in this directory as if he were running on NixOS.  Hopefully it is useful to others.  At worst, this documentation is useful for his purposes.

## Commands used to create the image

These commands are run from the root, the same directory as this README
```
docker pull nixos/nix
docker build -f Dockerfile --tag "harvest:nix" .
```

## Command used to create a container

```
docker run \
    --name harvest-nix \
    --hostname harvest \
    --interactive \
    --tty \
    `# maps pardir on host into docker guest` \
    --volume $(pwd):/home/harvest/harvest-code \
    --volume $(pwd)/../TRACTOR-pipeline-automation:/home/harvest/TRACTOR-pipeline-automation \
    --volume $(pwd)/../Code-Examples:/home/harvest/Code-Examples \
    `# This note and the accompanying ssh stuff is copied from immunant` \
    `# NOTE: ssh forwarding does not work with Docker for Mac ATM.` \
    `# More info here https://github.com/docker/for-mac/issues/483` \
    --volume $(dirname $SSH_AUTH_SOCK):$(dirname $SSH_AUTH_SOCK) \
    --env SSH_AUTH_SOCK=$SSH_AUTH_SOCK \
    --user root \
    harvest:nix
```

After creating the container, `cd` into `/home/harvest/harvest-code`.

Then run `nix-shell` to start up the shell with the needed packages.  This will have to be run every time the container is entered, not just on creation.

Use `cargo build` from within the nix-shell and that should succeed.

## Commands used to manage and/or connect to an existing container

Start (and Stop) Container
```
docker start harvest-nix
docker stop harvest-nix
```

Connect to a running container
```
docker exec --user root -it harvest-nix sh -c "export COLUMNS=`tput cols`; export LINES=`tput lines`; exec bash"
```

