use log::debug;

use crate::K8Config;

fn load_cert_auth() -> String {
    let k8_config = K8Config::load().expect("loading");

    let ctx = match k8_config {
        K8Config::Pod(_) => panic!("should not be pod"),
        K8Config::KubeConfig(ctx) => ctx,
    };

    let config = ctx.config;

    let cluster = config
        .current_cluster()
        .expect("should have current context");

    cluster
        .cluster
        .certificate_authority
        .as_ref()
        .clone()
        .expect("certificate authority")
        .to_string()
}

pub struct Option {
    ctx_name: String,
}

impl Default for Option {
    fn default() -> Self {
        Option {
            ctx_name: "flvkube".to_owned(),
        }
    }
}

/// create kube context that copy current cluster configuration
pub fn create_dns_context(option: Option) {
    const TEMPLATE: &'static str = r#"
#!/bin/bash
export IP=$(minikube ip)
sudo sed -i '' '/minikubeCA/d' /etc/hosts
echo "$IP minikubeCA" | sudo tee -a  /etc/hosts
cd ~
kubectl config set-cluster {{ name }} --server=https://minikubeCA:8443 --certificate-authority={{ ca }}
kubectl config set-context {{ name }} --user=minikube --cluster={{ name }}
kubectl config use-context {{ name }}
"#;

    use std::env;
    use std::fs::OpenOptions;
    use std::io;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::process::Command;

    use tera::Context;
    use tera::Tera;

    let mut tera = Tera::default();

    tera.add_raw_template("cube.sh", TEMPLATE)
        .expect("string compilation");

    let mut context = Context::new();
    context.insert("name", &option.ctx_name);
    context.insert("ca", &load_cert_auth());

    let render = tera.render("cube.sh", &context).expect("rendering");

    let tmp_file = env::temp_dir().join("flv_minikube.sh");

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o755)
        .open(tmp_file.clone())
        .expect("temp script can't be created");

    file.write_all(render.as_bytes())
        .expect("file write failed");

    file.sync_all().expect("sync");
    drop(file);

    debug!("script {}", render);

    let output = Command::new(tmp_file).output().expect("cluster command");
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();
}
