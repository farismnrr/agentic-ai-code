use hickory_resolver::config::*;
use hickory_resolver::TokioResolver;
fn main() {
    let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default());
}
