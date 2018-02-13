#!/bin/bash

red=`tput setaf 1`
green=`tput setaf 2`
reset=`tput sgr0`

function log() {
    echo ""
    echo "${green}$1${reset}"
    echo ""
}

function fail() {
    echo ""
    echo "${red}$1${reset}"
    echo ""
    exit 1
}

if [[ ! "$OSTYPE" =~ ^darwin ]]; then
    fail "This script only supports Mac OS"
fi;

if ! [ -x "$(command -v rustup)" ]; then
    log "Installing rust"
    curl -s https://static.rust-lang.org/rustup.sh | sh -s -- --channel=nightly
fi;

if [[ ! -d optirust ]]; then
    log "Cloning optirust in the 'optirust' folder, feel free to move it afterwards"
    git clone --depth=1 https://github.com/charignon/optirust
fi

if [[ ! -d pulp ]]; then
    log "Cloning the pulp repo in the 'pulp' folder, feel free to move it afterwards"
    git clone --depth=1 https://github.com/coin-or/pulp
fi;

# Any step that fails after this should fail the build
set -e
log "Installing optirust, this will take a while, go grab a coffee!"
pushd optirust
cargo build
log "Running the tests, they should pass"
cargo test
log "Creating sample input and config, see file $(pwd)/input and $(pwd)/config"
cat <<EOF > input
- title: title
  description: |
    this is my very very very
    long description for a
    very very interesting
    meeting
  attendees:
  - my_address@gmail.com
  min_date: 2018-02-05 10:00:00
  max_date: 2030-02-20 18:00:00
- title: title2
  description: description
  attendees:
  - my_address@gmail.com
  min_date: 2018-02-05 10:00:00
  max_date: 2030-02-20 18:00:00
EOF

cat <<EOF > config
small_rooms:
  - foo@bar.com
  - joe@baz.com
large_rooms:
  - bozorg@jam.com
EOF

log "Your optirust installation is at $(pwd)"
log "Please enable the google calendar API, see https://docs.google.com/presentation/d/16VBTYIqoqWAeS0PW6rmPSHqyaZk5cPCo1ABByqvClSg/edit?usp=sharing"
CBC_PATH="$(pwd)/src/pulp/solverdir/cbc/osx/64"
log "And add ${CBC_PATH} to your path, for example by adding the following to your shell config file: 'export PATH=\$PATH:${CBC_PATH}'"

