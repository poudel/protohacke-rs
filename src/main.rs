use std::env;
mod means_to_an_end;
mod primetime;
mod smoketest;

fn help() {
    println!(
        "usage:
proto <exercise-number>
"
    )
}

fn main() {
    // https://doc.rust-lang.org/rust-by-example/std_misc/arg/matching.html
    let args: Vec<String> = env::args().collect();

    match args.len() {
        2 => {
            let ex = &args[1];

            match ex.parse() {
                Ok(n) => {
                    let exercise_func = match n {
                        0 => smoketest::runserver,
                        1 => primetime::runserver,
                        2 => means_to_an_end::runserver,
                        // print help if it doesn't match
                        _ => help,
                    };
                    // run the exercise func
                    exercise_func();
                }
                Err(_) => {
                    help();
                    return;
                }
            };
        }

        _ => {
            help();
            return;
        }
    }
}
