mod consts;

fn test_manifest_contents(axml: &rusty_axml::parser::Axml) {
    let mut activities = rusty_axml::get_activities_names(&axml);
    activities.sort();
    assert_eq!(consts::ACTIVITIES, *activities);

    let mut services = rusty_axml::get_services_names(&axml);
    services.sort();
    assert_eq!(consts::SERVICES, *services);

    let mut providers = rusty_axml::get_providers_names(&axml);
    providers.sort();
    assert_eq!(consts::PROVIDERS, *providers);

    let mut receivers = rusty_axml::get_receivers_names(&axml);
    receivers.sort();
    assert_eq!(consts::RECEIVERS, *receivers);

    let mut requested_permissions = rusty_axml::get_requested_permissions(&axml);
    requested_permissions.sort();
    assert_eq!(consts::REQUESTED_PERMISSIONS, *requested_permissions);

    let mut declared_permissions = rusty_axml::get_declared_permissions(&axml);
    declared_permissions.sort();
    assert_eq!(consts::DECLARED_PERMISSIONS, *declared_permissions);
}

#[test]
fn test_from_apk() {
    let cursor = rusty_axml::create_cursor_from_apk("tests/assets/lumen.apk").unwrap();
    let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
    test_manifest_contents(&axml);
}

#[test]
fn test_from_axml() {
    let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
    let axml = rusty_axml::parse_from_cursor(cursor).unwrap();
    test_manifest_contents(&axml);
}

#[test]
fn test_component() {
    let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
    let axml = rusty_axml::parse_from_cursor(cursor).unwrap();

    let service = rusty_axml::find_node_by_name(&axml, "edu.berkeley.icsi.haystack.services.LocalVpnService").unwrap();
    let service = service.borrow();

    assert_eq!(service.get_attr("android:permission"), Some("android.permission.BIND_VPN_SERVICE"));
    assert_eq!(service.children().len(), 1);

    let filter = service.children().first().unwrap().clone();
    let filter = filter.borrow();
    assert_eq!(filter.element_type(), "intent-filter");
    assert_eq!(filter.children().len(), 1);

    let action = filter.children().first().unwrap().clone();
    let action = action.borrow();
    assert_eq!(action.get_name(), Some("android.net.VpnService"));
}

#[test]
fn test_manifest_attributes() {
    let cursor = rusty_axml::create_cursor_from_axml("tests/assets/AndroidManifest.xml").unwrap();
    let axml = rusty_axml::parse_from_cursor(cursor).unwrap();

    let node  = rusty_axml::find_nodes_by_type(&axml, "manifest").into_iter().next().unwrap();
    let node = node.borrow();

    assert_eq!(node.get_attr("package"), Some("edu.berkeley.icsi.haystack"));
    assert_eq!(node.get_attr("android:versionName"), Some("2.2"));
    assert_eq!(node.get_attr("android:versionCode"), Some("203"));
}
