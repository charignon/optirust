use types::{DesiredMeeting, Input};

#[allow(dead_code)]
pub fn test_input() -> String {
    "
meetings:
  - title: title
    description: description
    attendees:
    - laurent.charignon@foo.com
    - foo.bar@laurent.com
    min_date: 2018-02-08 10:00:00
    max_date: 2018-02-20 18:00:00
  - title: title 
    description: description 2
    attendees:
    - laurent.charignon@foo.com
    - contact@laurent.com
    min_date: 2018-02-08 11:00:00
    max_date: 2018-02-10 16:00:00
".to_string()
}

#[allow(dead_code)]
pub fn test_config() -> String {
    "
rooms:
  small:
    - name: Foo
      email: foo@bar.com
    - name: Bar
      email: joe@baz.com
  large:
    - name: Bozorg
      email: bozorg@jam.com
".to_string()
}

#[allow(dead_code)]
pub fn test_desired_meeting() -> DesiredMeeting {
    let a = Input::from_yaml_str(&test_input());
    return a.meetings[1].clone();
}
