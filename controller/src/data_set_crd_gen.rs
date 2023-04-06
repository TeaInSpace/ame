use ame::custom_resources::data_set::DataSet;
use kube::CustomResourceExt;

fn main() {
    print!("{}", serde_yaml::to_string(&DataSet::crd()).unwrap());
}
