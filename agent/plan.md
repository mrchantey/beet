

the `repeat.rs` behavior is incorrect, and demonstrates a broken part of our implementation.
Each repeat and repeat_times should be a parent with zero or one children.
they should call that child in a loop, until done. if the child returns Outcome::Fail, immediately break and propagate.
otherwise finally return the last Outcome::pass.
this means the initial Input must be Clone, passed in for each child call.
