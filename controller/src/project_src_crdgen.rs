use ame::custom_resources::project_source::ProjectSource;
use kube::CustomResourceExt;

fn main() {
    print!("{}", serde_yaml::to_string(&ProjectSource::crd()).unwrap())
}
