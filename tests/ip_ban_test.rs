/// Integration tests for IP ban functionality
/// Tests that banned IP addresses cannot log in and bans are enforced correctly
mod common;
use serial_test::serial;

use common::{database::*, fixtures::*};
use dumpster::web::login::{check_ip_ban, login, LoginResultStatus};

#[actix_rt::test]
#[serial]
async fn test_permanently_banned_ip_cannot_login() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a regular user
    let _user = create_test_user(&db, "ip_banned_user1", "correct_password")
        .await
        .expect("Failed to create user");

    // Create a permanent IP ban
    let banned_ip = "192.168.1.100";
    create_ip_ban(&db, banned_ip, "Testing IP ban", true, None, false)
        .await
        .expect("Failed to create IP ban");

    // Verify IP is banned
    let banned = is_ip_banned(&db, banned_ip)
        .await
        .expect("Failed to check IP ban status");
    assert!(banned, "IP should be banned");

    // Check IP ban directly
    let ban_info = check_ip_ban(banned_ip)
        .await
        .expect("Failed to check IP ban");
    assert!(ban_info.is_some(), "Should find IP ban");

    let ban = ban_info.unwrap();
    assert!(ban.is_permanent, "Ban should be permanent");
    assert_eq!(ban.reason, "Testing IP ban");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_temporarily_banned_ip_blocks_access() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a temporary IP ban (60 minutes)
    let banned_ip = "192.168.1.101";
    create_ip_ban(&db, banned_ip, "Temporary IP ban", false, Some(60), false)
        .await
        .expect("Failed to create IP ban");

    // Verify IP is banned
    let banned = is_ip_banned(&db, banned_ip)
        .await
        .expect("Failed to check IP ban status");
    assert!(banned, "IP should be banned");

    // Check IP ban directly
    let ban_info = check_ip_ban(banned_ip)
        .await
        .expect("Failed to check IP ban");
    assert!(ban_info.is_some(), "Should find IP ban");

    let ban = ban_info.unwrap();
    assert!(!ban.is_permanent, "Ban should not be permanent");
    assert!(ban.expires_at.is_some(), "Ban should have expiration");
    assert_eq!(ban.reason, "Temporary IP ban");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_expired_ip_ban_allows_access() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create an expired IP ban (expired 1 minute ago)
    let expired_ip = "192.168.1.102";
    create_ip_ban(&db, expired_ip, "Expired IP ban", false, Some(-1), false)
        .await
        .expect("Failed to create IP ban");

    // Verify IP is NOT banned (ban has expired)
    let banned = is_ip_banned(&db, expired_ip)
        .await
        .expect("Failed to check IP ban status");
    assert!(!banned, "Expired IP ban should not block access");

    // Check IP ban directly - should return None since ban expired
    let ban_info = check_ip_ban(expired_ip)
        .await
        .expect("Failed to check IP ban");
    assert!(ban_info.is_none(), "Expired ban should not be returned");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_non_banned_ip_can_access() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a user
    let _user = create_test_user(&db, "non_ip_banned_user", "correct_password")
        .await
        .expect("Failed to create user");

    // Check a non-banned IP
    let clean_ip = "192.168.1.103";
    let ban_info = check_ip_ban(clean_ip)
        .await
        .expect("Failed to check IP ban");
    assert!(ban_info.is_none(), "Non-banned IP should not have ban info");

    // Attempt login should succeed
    let result = login("non_ip_banned_user", "correct_password", &None::<String>)
        .await
        .expect("Login function failed");

    assert!(
        matches!(result.result, LoginResultStatus::Success),
        "Non-banned IP should allow login"
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_range_ban_is_stored_correctly() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a range ban (CIDR notation)
    let range_ip = "10.0.0.0/8";
    let ban = create_ip_ban(&db, range_ip, "Range ban test", true, None, true)
        .await
        .expect("Failed to create range ban");

    assert!(ban.is_range_ban, "Ban should be marked as range ban");
    assert_eq!(ban.ip_address, range_ip);

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_range_ban_blocks_ips_in_range() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create a range ban for 10.0.0.0/8 (covers 10.0.0.0 - 10.255.255.255)
    let range_ip = "10.0.0.0/8";
    create_ip_ban(&db, range_ip, "Range ban test", true, None, true)
        .await
        .expect("Failed to create range ban");

    // IPs within the range should be blocked
    let ip_in_range1 = "10.0.0.1";
    let ban_info1 = check_ip_ban(ip_in_range1)
        .await
        .expect("Failed to check IP ban");
    assert!(
        ban_info1.is_some(),
        "IP {} within range should be blocked",
        ip_in_range1
    );

    let ip_in_range2 = "10.255.255.254";
    let ban_info2 = check_ip_ban(ip_in_range2)
        .await
        .expect("Failed to check IP ban");
    assert!(
        ban_info2.is_some(),
        "IP {} within range should be blocked",
        ip_in_range2
    );

    let ip_in_range3 = "10.50.100.200";
    let ban_info3 = check_ip_ban(ip_in_range3)
        .await
        .expect("Failed to check IP ban");
    assert!(
        ban_info3.is_some(),
        "IP {} within range should be blocked",
        ip_in_range3
    );

    // IPs outside the range should NOT be blocked
    let ip_outside1 = "192.168.1.1";
    let ban_info_out1 = check_ip_ban(ip_outside1)
        .await
        .expect("Failed to check IP ban");
    assert!(
        ban_info_out1.is_none(),
        "IP {} outside range should NOT be blocked",
        ip_outside1
    );

    let ip_outside2 = "172.16.0.1";
    let ban_info_out2 = check_ip_ban(ip_outside2)
        .await
        .expect("Failed to check IP ban");
    assert!(
        ban_info_out2.is_none(),
        "IP {} outside range should NOT be blocked",
        ip_outside2
    );

    let ip_outside3 = "11.0.0.1";
    let ban_info_out3 = check_ip_ban(ip_outside3)
        .await
        .expect("Failed to check IP ban");
    assert!(
        ban_info_out3.is_none(),
        "IP {} outside range should NOT be blocked",
        ip_outside3
    );

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_ip_ban_reason_returned() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    let ban_reason = "Multiple abuse violations - third strike";
    let banned_ip = "192.168.1.104";

    // Create IP ban with specific reason
    create_ip_ban(&db, banned_ip, ban_reason, true, None, false)
        .await
        .expect("Failed to create IP ban");

    // Verify reason is included
    let ban_info = check_ip_ban(banned_ip)
        .await
        .expect("Failed to check IP ban");

    let ban = ban_info.expect("Should find IP ban");
    assert_eq!(ban.reason, ban_reason);
    assert!(ban.is_permanent);
    assert!(ban.expires_at.is_none());

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_different_ips_independent() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create users
    let _user1 = create_test_user(&db, "ip_test_user1", "password1")
        .await
        .expect("Failed to create user 1");
    let _user2 = create_test_user(&db, "ip_test_user2", "password2")
        .await
        .expect("Failed to create user 2");

    // Ban only one IP
    let banned_ip = "192.168.1.200";
    let clean_ip = "192.168.1.201";

    create_ip_ban(&db, banned_ip, "Ban test", true, None, false)
        .await
        .expect("Failed to create IP ban");

    // Banned IP should be blocked
    let ban_info = check_ip_ban(banned_ip)
        .await
        .expect("Failed to check banned IP");
    assert!(ban_info.is_some(), "Banned IP should be blocked");

    // Clean IP should be allowed
    let clean_info = check_ip_ban(clean_ip)
        .await
        .expect("Failed to check clean IP");
    assert!(clean_info.is_none(), "Clean IP should be allowed");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}

#[actix_rt::test]
#[serial]
async fn test_ipv6_ban() {
    let db = setup_test_database()
        .await
        .expect("Failed to connect to test database");

    cleanup_test_data(&db).await.expect("Failed to cleanup");

    // Create an IPv6 ban
    let ipv6_address = "2001:db8::1";
    create_ip_ban(&db, ipv6_address, "IPv6 ban test", true, None, false)
        .await
        .expect("Failed to create IPv6 ban");

    // Verify IPv6 is banned
    let ban_info = check_ip_ban(ipv6_address)
        .await
        .expect("Failed to check IPv6 ban");
    assert!(ban_info.is_some(), "IPv6 address should be banned");

    cleanup_test_data(&db).await.expect("Failed to cleanup");
}
