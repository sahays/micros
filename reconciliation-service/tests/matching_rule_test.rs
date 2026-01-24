//! Integration tests for matching rule operations.

mod common;

use common::{spawn_app, with_tenant};
use reconciliation_service::grpc::proto::*;
use uuid::Uuid;

#[tokio::test]
async fn create_matching_rule_with_contains_pattern() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Stripe Payouts".to_string(),
            description_pattern: "STRIPE PAYOUT".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: Some(1),
        },
        &app.tenant_id,
    );

    let response = client.create_matching_rule(request).await;
    assert!(response.is_ok(), "Expected OK, got: {:?}", response);

    let rule = response.unwrap().into_inner().rule.unwrap();
    assert_eq!(rule.name, "Stripe Payouts");
    assert_eq!(rule.description_pattern, "STRIPE PAYOUT");
    assert_eq!(rule.match_type, MatchType::Contains as i32);
    assert_eq!(rule.priority, 1);
    assert!(rule.is_active);
}

#[tokio::test]
async fn create_matching_rule_with_regex_pattern() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Rent Payment".to_string(),
            description_pattern: r"^RENT-\d+$".to_string(),
            match_type: MatchType::Regex.into(),
            target_account_id: None,
            priority: Some(5),
        },
        &app.tenant_id,
    );

    let response = client.create_matching_rule(request).await;
    assert!(response.is_ok());

    let rule = response.unwrap().into_inner().rule.unwrap();
    assert_eq!(rule.match_type, MatchType::Regex as i32);
}

#[tokio::test]
async fn create_matching_rule_with_invalid_regex_returns_error() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Bad Rule".to_string(),
            description_pattern: "[invalid(regex".to_string(), // Invalid regex
            match_type: MatchType::Regex.into(),
            target_account_id: None,
            priority: None,
        },
        &app.tenant_id,
    );

    let response = client.create_matching_rule(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn create_matching_rule_with_empty_name_returns_error() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let request = with_tenant(
        CreateMatchingRuleRequest {
            name: "".to_string(),
            description_pattern: "TEST".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: None,
        },
        &app.tenant_id,
    );

    let response = client.create_matching_rule(request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn get_matching_rule_returns_rule() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create a rule
    let create_request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Test Rule".to_string(),
            description_pattern: "TEST".to_string(),
            match_type: MatchType::Exact.into(),
            target_account_id: None,
            priority: Some(10),
        },
        &app.tenant_id,
    );

    let rule = client
        .create_matching_rule(create_request)
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();

    // Get the rule
    let get_request = with_tenant(
        GetMatchingRuleRequest {
            rule_id: rule.rule_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.get_matching_rule(get_request).await;
    assert!(response.is_ok());

    let fetched = response.unwrap().into_inner().rule.unwrap();
    assert_eq!(fetched.rule_id, rule.rule_id);
    assert_eq!(fetched.name, "Test Rule");
}

#[tokio::test]
async fn list_matching_rules_returns_rules_by_priority() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create rules with different priorities (out of order)
    for (name, priority) in [("Rule C", 10), ("Rule A", 1), ("Rule B", 5)] {
        let request = with_tenant(
            CreateMatchingRuleRequest {
                name: name.to_string(),
                description_pattern: "PATTERN".to_string(),
                match_type: MatchType::Contains.into(),
                target_account_id: None,
                priority: Some(priority),
            },
            &app.tenant_id,
        );
        client.create_matching_rule(request).await.unwrap();
    }

    // List rules
    let list_request = with_tenant(
        ListMatchingRulesRequest {
            page_size: 10,
            page_token: None,
            active_only: None,
        },
        &app.tenant_id,
    );

    let response = client.list_matching_rules(list_request).await.unwrap();
    let rules = response.into_inner().rules;

    assert_eq!(rules.len(), 3);
    // Should be ordered by priority ascending
    assert_eq!(rules[0].name, "Rule A"); // priority 1
    assert_eq!(rules[1].name, "Rule B"); // priority 5
    assert_eq!(rules[2].name, "Rule C"); // priority 10
}

#[tokio::test]
async fn list_matching_rules_active_only_filter() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create two rules
    let create_request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Active Rule".to_string(),
            description_pattern: "ACTIVE".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: Some(1),
        },
        &app.tenant_id,
    );
    client.create_matching_rule(create_request).await.unwrap();

    let create_request2 = with_tenant(
        CreateMatchingRuleRequest {
            name: "Will Be Inactive".to_string(),
            description_pattern: "INACTIVE".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: Some(2),
        },
        &app.tenant_id,
    );
    let rule2 = client
        .create_matching_rule(create_request2)
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();

    // Deactivate the second rule
    let update_request = with_tenant(
        UpdateMatchingRuleRequest {
            rule_id: rule2.rule_id,
            name: None,
            description_pattern: None,
            match_type: None,
            target_account_id: None,
            priority: None,
            is_active: Some(false),
        },
        &app.tenant_id,
    );
    client.update_matching_rule(update_request).await.unwrap();

    // List active only
    let list_request = with_tenant(
        ListMatchingRulesRequest {
            page_size: 10,
            page_token: None,
            active_only: Some(true),
        },
        &app.tenant_id,
    );

    let response = client.list_matching_rules(list_request).await.unwrap();
    let rules = response.into_inner().rules;

    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "Active Rule");
}

#[tokio::test]
async fn update_matching_rule_changes_fields() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create a rule
    let create_request = with_tenant(
        CreateMatchingRuleRequest {
            name: "Original Name".to_string(),
            description_pattern: "ORIGINAL".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: Some(1),
        },
        &app.tenant_id,
    );

    let rule = client
        .create_matching_rule(create_request)
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();

    // Update the rule
    let update_request = with_tenant(
        UpdateMatchingRuleRequest {
            rule_id: rule.rule_id.clone(),
            name: Some("Updated Name".to_string()),
            description_pattern: Some("UPDATED".to_string()),
            match_type: Some(MatchType::Exact.into()),
            target_account_id: None,
            priority: Some(5),
            is_active: None,
        },
        &app.tenant_id,
    );

    let response = client.update_matching_rule(update_request).await;
    assert!(response.is_ok());

    let updated = response.unwrap().into_inner().rule.unwrap();
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description_pattern, "UPDATED");
    assert_eq!(updated.match_type, MatchType::Exact as i32);
    assert_eq!(updated.priority, 5);
}

#[tokio::test]
async fn delete_matching_rule_removes_rule() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    // Create a rule
    let create_request = with_tenant(
        CreateMatchingRuleRequest {
            name: "To Delete".to_string(),
            description_pattern: "DELETE".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: None,
        },
        &app.tenant_id,
    );

    let rule = client
        .create_matching_rule(create_request)
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();

    // Delete the rule
    let delete_request = with_tenant(
        DeleteMatchingRuleRequest {
            rule_id: rule.rule_id.clone(),
        },
        &app.tenant_id,
    );

    let response = client.delete_matching_rule(delete_request).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().success);

    // Verify rule is gone
    let get_request = with_tenant(
        GetMatchingRuleRequest {
            rule_id: rule.rule_id,
        },
        &app.tenant_id,
    );

    let response = client.get_matching_rule(get_request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn tenant_isolation_for_rules() {
    let app = spawn_app().await;
    let mut client = app.grpc_client.clone();

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    // Create rule for tenant1
    let request1 = with_tenant(
        CreateMatchingRuleRequest {
            name: "Tenant1 Rule".to_string(),
            description_pattern: "T1".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: None,
        },
        &tenant1,
    );

    let rule1 = client
        .create_matching_rule(request1)
        .await
        .unwrap()
        .into_inner()
        .rule
        .unwrap();

    // Create rule for tenant2
    let request2 = with_tenant(
        CreateMatchingRuleRequest {
            name: "Tenant2 Rule".to_string(),
            description_pattern: "T2".to_string(),
            match_type: MatchType::Contains.into(),
            target_account_id: None,
            priority: None,
        },
        &tenant2,
    );
    client.create_matching_rule(request2).await.unwrap();

    // Tenant1 should only see their rule
    let list_request1 = with_tenant(
        ListMatchingRulesRequest {
            page_size: 10,
            page_token: None,
            active_only: None,
        },
        &tenant1,
    );

    let list1 = client.list_matching_rules(list_request1).await.unwrap();
    assert_eq!(list1.into_inner().rules.len(), 1);

    // Tenant2 trying to get tenant1's rule should fail
    let get_request = with_tenant(
        GetMatchingRuleRequest {
            rule_id: rule1.rule_id,
        },
        &tenant2,
    );

    let response = client.get_matching_rule(get_request).await;
    assert!(response.is_err());
    assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
}
