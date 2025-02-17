//! Ground truth data for the test app

pub const REQUESTED_PERMISSIONS: [&str; 3] = [
    "android.permission.ACCESS_COARSE_LOCATION",
    "android.permission.ACCESS_FINE_LOCATION",
    "eu.jgamba.myapplication.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
];

pub const DECLARED_PERMISSIONS: [&str; 2] = [
    "eu.jgamba.MY_PERMISSION",
    "eu.jgamba.myapplication.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION",
];

pub const ACTIVITIES: [&str; 1] = [
    "eu.jgamba.myapplication.MainActivity",
];

pub const SERVICES: [&str; 2] = [
    "eu.jgamba.myapplication.MyFirstService",
    "eu.jgamba.myapplication.MySecondService",
];

pub const PROVIDERS: [&str; 2] = [
    "androidx.startup.InitializationProvider",
    "eu.jgamba.myapplication.MyContentProvider",
];

pub const RECEIVERS: [&str; 2] = [
    "androidx.profileinstaller.ProfileInstallReceiver",
    "eu.jgamba.myapplication.MyReceiver",
];
