use libra_types::{test_drop_helper::DropTemp, legacy_types::app_cfg::AppCfg, exports::AuthenticationKey};
use zapatos_smoke_test::smoke_test_environment::{
  new_local_swarm_with_release,
};

use libra_framework::release::ReleaseTarget;
use libra_tower::core::{backlog, proof};
use zapatos_forge::Swarm;

/// Testing that we can get a swarm up with the current head.mrb
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn tower_genesis() {

    let release = ReleaseTarget::Head.load_bundle().unwrap();
    let mut swarm = new_local_swarm_with_release(4, release).await;

    let info = swarm.aptos_public_info_for_node(0);
    let url = info.url().to_string();

    let node = swarm.validators().into_iter().next().unwrap();

    // let local = LocalAccount::new(node.peer_id(), node.account_private_key().unwrap().private_key(), 0);
    // let info = swarm.aptos_public_info();
    
    // let port = swarm.validators().next().unwrap().port();
    // let url = Url::from_str(&format!("http://localhost:{}", port));

    let mut app_cfg = AppCfg::init_app_configs(
      AuthenticationKey::ed25519(&node.account_private_key().as_ref().unwrap().public_key()),
      node.peer_id(),
      None,
      None,
    ).unwrap();

    let temp_files = &DropTemp::new_in_crate("_smoke_test_temp").0;

    app_cfg.workspace.node_home = temp_files.to_owned();
    app_cfg.profile.upstream_nodes = vec![url.parse().unwrap()];
    app_cfg.profile.test_private_key = Some(node.account_private_key().as_ref().unwrap().private_key());

    let _proof = proof::write_genesis(&app_cfg).expect("could not write genesis proof");

    // dbg!(&proof);

    backlog::process_backlog(&app_cfg).await.unwrap();


    // next_proof::get_next_proof_params_from_local(config)?


}
