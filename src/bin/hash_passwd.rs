use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    password: String,
}

fn main() -> ergo::error::Result<()> {
    let args = Args::from_args();
    let hash = ergo::auth::password::new_hash(&args.password)?;
    println!("{}", hash);
    Ok(())
}
