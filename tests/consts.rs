//! Ground truth data for the test app

pub const REQUESTED_PERMISSIONS: [&str; 18] = [
    "android.permission.ACCESS_COARSE_LOCATION",
    "android.permission.ACCESS_FINE_LOCATION",
    "android.permission.ACCESS_NETWORK_STATE",
    "android.permission.ACCESS_WIFI_STATE",
    "android.permission.AUTHENTICATE_ACCOUNTS",
    "android.permission.CHANGE_NETWORK_STATE",
    "android.permission.CHANGE_WIFI_STATE",
    "android.permission.GET_ACCOUNTS",
    "android.permission.GET_TASKS",
    "android.permission.INTERNET",
    "android.permission.PROCESS_OUTGOING_CALLS",
    "android.permission.READ_CALL_LOG",
    "android.permission.READ_CONTACTS",
    "android.permission.READ_PHONE_STATE",
    "android.permission.READ_PROFILE",
    "android.permission.READ_SMS",
    "android.permission.RECEIVE_BOOT_COMPLETED",
    "android.permission.WRITE_EXTERNAL_STORAGE",
];

pub const DECLARED_PERMISSIONS: [&str; 0] = [ ];

pub const ACTIVITIES: [&str; 17] = [
    "edu.berkeley.icsi.haystack.activities.AppProfileActivity",
    "edu.berkeley.icsi.haystack.activities.ContactInfoActivity",
    "edu.berkeley.icsi.haystack.activities.CustomKeywordsActivity",
    "edu.berkeley.icsi.haystack.activities.DatabaseManagementActivity",
    "edu.berkeley.icsi.haystack.activities.DispatcherActivity",
    "edu.berkeley.icsi.haystack.activities.FirewallManager",
    "edu.berkeley.icsi.haystack.activities.FirewallManagerCustomDomainProfile",
    "edu.berkeley.icsi.haystack.activities.FirewallManagerDomainProfile",
    "edu.berkeley.icsi.haystack.activities.HaystackNotificationsActivity",
    "edu.berkeley.icsi.haystack.activities.MainActivity",
    "edu.berkeley.icsi.haystack.activities.PIILeakDetailActivity",
    "edu.berkeley.icsi.haystack.activities.SettingsActivity",
    "edu.berkeley.icsi.haystack.activities.intro.DataGenerator",
    "edu.berkeley.icsi.haystack.activities.intro.IntroHaystackActivity",
    "edu.berkeley.icsi.haystack.activities.intro.IntroHaystackActivityFourth",
    "edu.berkeley.icsi.haystack.activities.intro.IntroHaystackActivitySecond",
    "edu.berkeley.icsi.haystack.activities.intro.IntroHaystackActivityThird",

];

pub const SERVICES: [&str; 5] = [
    "edu.berkeley.icsi.haystack.database.ComputeHashesService",
    "edu.berkeley.icsi.haystack.services.IntelligenceService",
    "edu.berkeley.icsi.haystack.services.LocalVpnService",
    "edu.berkeley.icsi.haystack.services.TLSProxyService",
    "edu.berkeley.icsi.haystack.services.TrafficAnalyzerService",
];

pub const PROVIDERS: [&str; 0] = [ ];

pub const RECEIVERS: [&str; 5] = [
    "edu.berkeley.icsi.haystack.receivers.BootReceiver",
    "edu.berkeley.icsi.haystack.receivers.CallListener",
    "edu.berkeley.icsi.haystack.receivers.ConnectivityReceiver",
    "edu.berkeley.icsi.haystack.receivers.SmsReceiver",
    "edu.berkeley.icsi.haystack.receivers.TrafficAnalyzerDBAlarmReceiver",
];
