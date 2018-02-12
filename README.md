# Optirust

Optimized meeting scheduling with Google Calendar API

## Installation

- Install rust with rustup
- Download or build the cbc solver (you can use the one from https://github.com/coin-or/pulp)
- Clone this repo.
- Generate credential for the google calendar api, put them in `client_secret.json` (https://docs.google.com/presentation/d/16VBTYIqoqWAeS0PW6rmPSHqyaZk5cPCo1ABByqvClSg/edit?usp=sharing)

## Usage
- Write a config file with email address for rooms you want to book (small is 1-2 people, large is 3+):
```yaml
rooms:
  small:
    - name: Foo
      email: foo@bar.com
    - name: Bar
      email: joe@baz.com
  large:
    - name: Bozorg
      email: bozorg@jam.com
```

- Write an input file with the meetings you would like to book:
```yaml
meetings:
  - title: title
    description: description
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
```

- Run the code in dry run mode: `cargo run -- --input input --config config` (it will print the meeting that would be booked if you ran it with the `--book` flag)
- Book the meetings for real: `cargo run -- --book --input input --config config`
