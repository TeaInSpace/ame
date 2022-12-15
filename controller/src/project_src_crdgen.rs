use controller::project_source::ProjectSource;
use kube::CustomResourceExt;

fn main() {
    print!("{}", serde_yaml::to_string(&ProjectSource::crd()).unwrap())
}
