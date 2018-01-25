

/* Project Optirust

Goal: Implement calendar optimized scheduler using rust

What are the features that I want to build
1. we can parse the input file and map to a struct
2. we can generate objects for the attendees and rooms from the struct
3. we can populate those with their calendars
4. we can generate the possible meetings and score them
5. we can generate the equation system with everything and solve it
6. we can display a solution

Constraint: 10 lines per function
Constraint: 100% test coverage
Constraint: Timezone aware
Constraint: as much FP as possible, pluggable I/O and dependency, the middle should be
as isolated from the rest as possible

extern crate bio;
use bio::data_structures::interval_tree::{IntervalTree};
use bio::utils::Interval;
fn main() {
let mut tree = IntervalTree::new();
tree.insert(11..20, "Range_1");
tree.insert(25..30, "Range_2");
for r in tree.find(15..25) {
assert_eq!(r.interval().start, 11);
assert_eq!(r.interval().end, 20);
assert_eq!(r.interval(), &(Interval::from(11..20)));
assert_eq!(r.data(), &"Range_1");
    }
}

1. we can parse the input file and map to a struct
==================================================
What is the input?
We need two files, a config file with the paramater and an input file with the
requested meeting.


*/

#[test]
fn foo() {
    assert_eq!(4, 4);
}







