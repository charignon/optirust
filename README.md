# Optirust

Optimized meeting scheduling with Google Calendar API

## Automatic Installation

Go to a folder of your choosing and run:
```
curl https://raw.githubusercontent.com/charignon/optirust/master/installer.sh | sh
```

## Manual installation
- Install rust with rustup (https://www.rustup.rs/)
- Download or build the cbc solver (you can use the one from https://github.com/coin-or/pulp)
- Clone this repo.
- Generate credential for the google calendar api, put them in `client_secret.json` (https://docs.google.com/presentation/d/16VBTYIqoqWAeS0PW6rmPSHqyaZk5cPCo1ABByqvClSg/edit?usp=sharing)

## Using with Docker

See https://hub.docker.com/r/lcharignon/optirust

## Building as a docker image and running as a container

See https://hub.docker.com/r/lcharignon/optirust

## Usage
- Write a config file containing email addresses for the rooms you are allowed to book (small is 1-2 people, large is 3+):
```yaml
small_rooms:
  - foo@bar.com
  - joe@baz.com
large_rooms:
  - bozorg@jam.com
reject_iso_weekday:
  - 3 # Wednesday
  - 6 # Saturday
  - 7 # Sunday
reject_hour_range:
  # Lunch
  - from: 
      hours: 12
      minutes: 0
      seconds: 0
    to: 
      hours: 13
      minutes: 0
      seconds: 0
ignore_all_day_events: true
ignore_meetings_with_no_response: true
```

- Write an input file with the meetings you would like to book:
```yaml
- title: title
  description: |
    this is my very very very
    long description for a
    very very interesting
    meeting
  attendees:
  - my_address@gmail.com
  min_date: 2018-02-05T10:00:00
  max_date: 2030-02-20T18:00:00
- title: title2
  description: description
  attendees:
  - my_address@gmail.com
  min_date: 2018-02-05T10:00:00
  max_date: 2030-02-20T18:00:00
```

Optirust can help you schedule these meetings optimally and find rooms to host them:
- Dry run mode (no booking): `cargo run -- --input input --config config` (it will print the meeting that would be booked if you ran it with the `--book` flag)
- Book the meetings with google API: `cargo run -- --book --input input --config config`
