# optirust

Optimized meeting scheduling with Google Calendar API

## How to use

- Install rust with rustup
- Download or build the cbc solver.
- Clone this repo.
- Generate credential for the google calendar api, put them in =client_secret.json=.
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

- Write an input file with the meeting you want to book:
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

- Run the code in dry run mode, make sure that it works: `cargo run -- --input input --config config`
- Book the meetings for real: `cargo run -- --book --input input --config config`
