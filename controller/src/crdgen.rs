use ame::custom_resources::new_task::Task;
use kube::CustomResourceExt;

fn main() {
    print!("{}", serde_yaml::to_string(&Task::crd()).unwrap());
}
