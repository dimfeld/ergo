use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Args {
    password: String,
}

pub fn main(args: Args) -> crate::error::Result<()> {
    let hash = ergo_auth::password::new_hash(&args.password)?;
    println!("{}", hash);
    Ok(())
}
