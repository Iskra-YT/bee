mod file;
mod yaml;

fn main() {
    let path = String::from("./bee");
    let tasks = yaml::parse_yaml_files(&path).unwrap();
    for task_data in tasks {
        if let Some((name, task)) = task_data.iter().next() {
            println!("{}:", name);
            println!("  depends_on: {:?}", task.depends_on);
            println!("  run: {}", task.run);
        }
    }
}
