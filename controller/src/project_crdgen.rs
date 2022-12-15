use controller::project::Project;
use kube::CustomResourceExt;
fn main() {
    print!("{}", serde_yaml::to_string(&Project::crd()).unwrap())
}
