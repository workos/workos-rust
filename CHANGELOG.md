# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0](https://github.com/workos/workos-rust/compare/v2.5.0...v3.0.0) (2026-07-22)

* [#126](https://github.com/workos/workos-rust/pull/126) fix(generated): regenerate from spec

  **Features**
  * **[audit_logs](https://workos.com/docs/reference/audit-logs)**:
    * Added `expired` to `AuditLogExportState`

* [#129](https://github.com/workos/workos-rust/pull/129) feat(generated)!: regenerate from spec (2 changes)

  **Features**
  * **agents**:
    * Added model `ClaimViewResponse`
    * Added model `ClaimViewResponseOrganization`
    * Added model `AgentAdminLinkClaimAttemptToExternalUserRequest`
    * Added model `AgentAdminLinkClaimAttemptToExternalUserRequestUser`
    * Added enum `ClaimViewResponseStatus`
    * Added endpoint `PATCH /agents/claims/attempts`
    * Added model `AgentRegistration`
    * Added model `AgentCredentialValidation`
    * Added model `AgentRegistrationAgentIdentity`
    * Added model `AgentRegistrationClaim`
    * Added model `AgentAdminValidateCredentialRequest`
    * Added model `AgentRegistrationClaimClaimCompletion`
    * Added enum `AgentRegistrationStatus`
    * Added enum `AgentRegistrationKind`
    * Added enum `AgentAdminValidateCredentialRequestType`
    * Added service `Agents`
  * **[api_keys](https://workos.com/docs/reference/authkit/api-keys)**:
    * Added `agent_registration_id` to `ApiKeyValidationResponse`
  * **[connect](https://workos.com/docs/reference/workos-connect/standalone)**:
    * Added enum `ApplicationsRegistrationTypes`
    * Added parameter `Applications.list.registration_types`
  * **[directory_sync](https://workos.com/docs/reference/directory-sync)**:
    * Added parameter `DirectoryUsers.list.idp_id`
    * Added parameter `DirectoryUsers.list.email`
  * **[organizations](https://workos.com/docs/reference/organization)**:
    * Added model `OrganizationAuthorizedConnectApplicationList`
    * Added model `OrganizationAuthorizedConnectApplicationListData`
    * Added model `OrganizationAuthorizedConnectApplicationListListMetadata`
    * Added service `OrganizationsAuthorizedApplications`
  * **[pipes](https://workos.com/docs/reference/pipes)**:
    * Added model `DataIntegrationInstallation`
    * Added `auth_methods` to `CreateDataIntegration`
    * Added `api_key` to `CreateDataIntegration`
    * Added `api_key` to `UpdateDataIntegration`
    * Added `auth_methods` to `DataIntegration`
    * Added `installation` to `DataIntegration`
    * Added enum `CreateDataIntegrationAuthMethods`
    * Added enum `DataIntegrationAuthMethods`
    * Added model `DataIntegrationCredentialsResponse`
    * Added model `DataIntegrationCredentialsResponseCredential`
    * Added model `DataIntegrationsUpsertApiKeyRequest`
    * Added model `DataIntegrationsVendCredentialsRequest`
    * Added enum `DataIntegrationCredentialsResponseError`
    * Added endpoint `PUT /data-integrations/{slug}/api-key`
    * Added endpoint `POST /data-integrations/{slug}/credentials`
  * **[sso](https://workos.com/docs/reference/sso)**:
    * Added parameter `SSO.authorize.prompt`
  * **[user_management](https://workos.com/docs/reference/authkit/user)**:
    * Added `ssha256` to `CreateUserPasswordHashType`
    * Added `ssha256` to `UpdateUserPasswordHashType`
    * Added endpoint `GET /user_management/radar_challenges/{id}`
  * **[webhooks](https://workos.com/docs/reference/webhooks)**:
    * Added `agent.registration.revoked` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.revoked` to `UpdateWebhookEndpointEvents`
    * Added `agent.registration.deleted` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.deleted` to `UpdateWebhookEndpointEvents`
    * Added `radar.challenge_created` to `CreateWebhookEndpointEvents`
    * Added `radar.challenge_created` to `UpdateWebhookEndpointEvents`
    * Added `agent.registration.expired` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.expired` to `UpdateWebhookEndpointEvents`
  * **[widgets](https://workos.com/docs/reference/widgets)**:
    * Made `WidgetSessionToken.organization_id` optional

* [#130](https://github.com/workos/workos-rust/pull/130) feat(generated)!: regenerate from spec (5 changes)

  **⚠️ Breaking**
  * **[admin_portal](https://workos.com/docs/reference/admin-portal)**:
    * SDK surface change: Symbol "IntentOptions" was removed
  * **[connect](https://workos.com/docs/reference/workos-connect/standalone)**:
    * SDK surface change: Symbol "ConnectedAccountDto" was removed
  * **[organization_domains](https://workos.com/docs/reference/domain-verification)**:
    * SDK surface change: Symbol "DomainVerificationIntentOptions" was removed
  * **[pipes](https://workos.com/docs/reference/pipes)**:
    * SDK surface change: Symbol "DataIntegrationCredentialsDto" was removed
  * **[sso](https://workos.com/docs/reference/sso)**:
    * SDK surface change: Symbol "SSOIntentOptions" was removed

## [2.5.0](https://github.com/workos/workos-rust/compare/v2.4.0...v2.5.0) (2026-07-06)

* [#124](https://github.com/workos/workos-rust/pull/124) fix(generated): regenerate from spec

  **Features**
  * **[user_management](https://workos.com/docs/reference/authkit/user)**:
    * Added model `UserRoleAssignmentSource`
    * Added `source` to `UserRoleAssignment`
    * Added enum `UserRoleAssignmentSourceType`
    * Added parameter `UserManagementAuthentication.authorize.max_age`
    * Added endpoint `GET /user_management/cors_origins`
    * Added endpoint `GET /user_management/redirect_uris`

  **Fixes**
  * Restore mistakenly removed CreateMagicAuth logic from previous release

## [2.4.0](https://github.com/workos/workos-rust/compare/v2.3.0...v2.4.0) (2026-07-02)

* [#118](https://github.com/workos/workos-rust/pull/118) fix(generated): regenerate from spec

  **Features**
  * **[pipes](https://workos.com/docs/reference/pipes)**:
    * Added model `DataIntegrationCredentialsResponse`
    * Added model `DataIntegrationCredentialsResponseCredential`
    * Added model `DataIntegrationsUpsertApiKeyRequest`
    * Added model `DataIntegrationsVendCredentialsRequest`
    * Added enum `DataIntegrationCredentialsResponseError`
    * Added endpoint `PUT /data-integrations/{slug}/api-key`
    * Added endpoint `POST /data-integrations/{slug}/credentials`

* [#120](https://github.com/workos/workos-rust/pull/120) fix(generated): regenerate from spec

  **⚠️ Breaking**
  * **[user_management](https://workos.com/docs/reference/authkit/user)**:
    * Removed model `SessionReauthenticated`
    * Removed model `SessionReauthenticatedData`
    * Removed model `SessionReauthenticatedDataImpersonator`
    * Removed enum `SessionReauthenticatedDataAuthMethod`
    * Removed enum `SessionReauthenticatedDataStatus`

  **Features**
  * **[webhooks](https://workos.com/docs/reference/webhooks)**:
    * Added `agent.registration.created` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.claim.attempt.created` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.claim.completed` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.credential.issued` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.organization.switched` to `CreateWebhookEndpointEvents`
    * Added `authentication.reauthentication_succeeded` to `CreateWebhookEndpointEvents`
    * Added `agent.registration.created` to `UpdateWebhookEndpointEvents`
    * Added `agent.registration.claim.attempt.created` to `UpdateWebhookEndpointEvents`
    * Added `agent.registration.claim.completed` to `UpdateWebhookEndpointEvents`
    * Added `agent.registration.credential.issued` to `UpdateWebhookEndpointEvents`
    * Added `agent.registration.organization.switched` to `UpdateWebhookEndpointEvents`
    * Added `authentication.reauthentication_succeeded` to `UpdateWebhookEndpointEvents`
  * **[webhooks](https://workos.com/docs/reference/webhooks)**:
    * Added `session.reauthenticated` to `CreateWebhookEndpointEvents`
    * Added `session.reauthenticated` to `UpdateWebhookEndpointEvents`
  * **[webhooks](https://workos.com/docs/reference/webhooks)**:
    * Added `pipes.connected_account.connection_failed` to `CreateWebhookEndpointEvents`
    * Added `pipes.connected_account.connection_failed` to `UpdateWebhookEndpointEvents`
  * **[user_management](https://workos.com/docs/reference/authkit/user)**:
    * Added model `UserRoleAssignmentSource`
    * Added `source` to `UserRoleAssignment`
    * Added enum `UserRoleAssignmentSourceType`
    * Added parameter `UserManagementAuthentication.authorize.max_age`
    * Added endpoint `GET /user_management/cors_origins`
    * Added endpoint `GET /user_management/redirect_uris`
  * **[audit_logs](https://workos.com/docs/reference/audit-logs)**:
    * Changed the format of `AuditLogExportCreation.range_start`
    * Changed the format of `AuditLogExportCreation.range_end`
  * **[audit_logs](https://workos.com/docs/reference/audit-logs)**:
    * Added `expired` to `AuditLogExportState`

  **Fixes**
  * **[admin_portal](https://workos.com/docs/reference/admin-portal)**:
    * Removed `intent_options` from `GenerateLink`
  * **[webhooks](https://workos.com/docs/reference/webhooks)**:
    * Removed `session.reauthenticated` from `CreateWebhookEndpointEvents`
    * Removed `session.reauthenticated` from `UpdateWebhookEndpointEvents`

* [#122](https://github.com/workos/workos-rust/pull/122) feat(generated): regenerate from spec (1 change)

  **Features**
  * **[pipes](https://workos.com/docs/reference/pipes)**:
    * Added model `DataIntegrationCredentialsDto`
    * Added model `CustomProviderDefinition`
    * Added model `CreateDataIntegration`
    * Added model `UpdateCustomProviderDefinition`
    * Added model `UpdateDataIntegration`
    * Added model `DataIntegration`
    * Added model `DataIntegrationList`
    * Added model `DataIntegrationListListMetadata`
    * Added model `DataIntegrationCredential`
    * Added model `DataIntegrationCustomProvider`
    * Added enum `DataIntegrationCredentialsType`
    * Added enum `CustomProviderDefinitionAuthenticateVia`
    * Added enum `UpdateCustomProviderDefinitionAuthenticateVia`
    * Added enum `DataIntegrationState`
    * Added enum `DataIntegrationCredentialType`
    * Added enum `DataIntegrationCustomProviderAuthenticateVia`
    * Added endpoint `GET /data-integrations`
    * Added endpoint `POST /data-integrations`
    * Added endpoint `GET /data-integrations/{slug}`
    * Added endpoint `PUT /data-integrations/{slug}`
    * Added endpoint `DELETE /data-integrations/{slug}`
    * Added endpoint `POST /user_management/users/{user_id}/connected_accounts/{slug}`
    * Added endpoint `PUT /user_management/users/{user_id}/connected_accounts/{slug}`

* [#123](https://github.com/workos/workos-rust/pull/123) feat(generated): regenerate from spec (2 changes)

  **Features**
  * **[user_management](https://workos.com/docs/reference/authkit/user)**:
    * Added model `SendRadarSmsChallenge`
    * Added model `SendRadarSmsChallengeResponse`
    * Added model `UrnWorkosOAuthGrantTypeRadarEmailChallengeCodeSessionAuthenticateRequest`
    * Added model `UrnWorkosOAuthGrantTypeRadarSmsChallengeCodeSessionAuthenticateRequest`
    * Added model `MagicAuthSendMagicAuthCodeAndReturnResponse`
    * Added model `UserCreateResponse`
    * Added `ip_address` to `CreateMagicCodeAndReturn`
    * Added `user_agent` to `CreateMagicCodeAndReturn`
    * Added `radar_auth_attempt_id` to `CreateMagicCodeAndReturn`
    * Added `signals_id` to `CreateMagicCodeAndReturn`
    * Added `ip_address` to `CreateUser`
    * Added `user_agent` to `CreateUser`
    * Added `signals_id` to `CreateUser`
    * Added `signals_id` to `AuthorizationCodeSessionAuthenticateRequest`
    * Added `signals_id` to `PasswordSessionAuthenticateRequest`
    * Added `radar_auth_attempt_id` to `PasswordSessionAuthenticateRequest`
    * Added `radar_auth_attempt_id` to `UrnWorkosOAuthGrantTypeMagicAuthCodeSessionAuthenticateRequest`
    * Added endpoint `POST /user_management/radar_challenges`
  * **[radar](https://workos.com/docs/reference/radar)**:
    * Added `signals_id` to `RadarStandaloneAssessRequest`

  **Fixes**
  * **[user_management](https://workos.com/docs/reference/authkit/user)**:
    * Changed request body for `UserManagementAuthentication.authenticate`
    * Changed response of `UserManagementUsers.create` from `User` to `UserCreateResponse`
    * Changed response of `UserManagementMagicAuth.sendMagicAuthCodeAndReturn` from `MagicAuth` to `MagicAuthSendMagicAuthCodeAndReturnResponse`

## [2.3.0](https://github.com/workos/workos-rust/compare/v2.2.0...v2.3.0) (2026-06-30)

* [#114](https://github.com/workos/workos-rust/pull/114) fix(generated): regenerate from spec

  **Fixes**
  * **[organization_membership](https://workos.com/docs/reference/authkit/organization-membership)**:
    * Added `roles` to organization membership models

## [2.2.0](https://github.com/workos/workos-rust/compare/v2.1.0...v2.2.0) (2026-06-23)

- [#111](https://github.com/workos/workos-rust/pull/111) feat(generated)!: regenerate from spec (11 changes)

  **Features**
  - **[authorization](https://workos.com/docs/reference/fga)**:
    - Added model `ReplaceGroupRoleAssignmentEntry`
    - Added model `ReplaceGroupRoleAssignments`
    - Added model `DeleteGroupRoleAssignmentsByCriteria`
    - Added endpoint `POST /authorization/groups/{group_id}/role_assignments`
    - Added endpoint `PUT /authorization/groups/{group_id}/role_assignments`
    - Added endpoint `DELETE /authorization/groups/{group_id}/role_assignments`
    - Added endpoint `GET /authorization/groups/{group_id}/role_assignments/{role_assignment_id}`
    - Added endpoint `DELETE /authorization/groups/{group_id}/role_assignments/{role_assignment_id}`
  - **[client](https://workos.com/docs/reference)**:
    - Added model `ClientApiToken`
    - Added model `ClientApiTokenResponse`
    - Added service `Client`
  - **[connect](https://workos.com/docs/reference/workos-connect/standalone)**:
    - Added `auth_method` to `ConnectedAccount`
    - Added `api_key_last_4` to `ConnectedAccount`
    - Added enum `ConnectedAccountAuthMethod`
  - **[groups](https://workos.com/docs/reference/groups)**:
    - Added model `CreateGroupRoleAssignment`
    - Added model `GroupRoleAssignment`
    - Added model `GroupRoleAssignmentList`
    - Added model `GroupRoleAssignmentResource`
  - **[organization_membership](https://workos.com/docs/reference/authkit/organization-membership)**:
    - Added model `UserOrganizationMembershipList`
    - Added model `UserOrganizationMembershipListListMetadata`
  - **[pipes](https://workos.com/docs/reference/pipes)**:
    - Added model `DataIntegrationCredentials`
    - Added model `DataIntegrationConfigurationResponse`
    - Added model `DataIntegrationConfigurationListResponse`
    - Added model `ConfigureDataIntegrationBody`
    - Added `auth_methods` to `DataIntegrationsListResponseData`
    - Added `auth_method` to `DataIntegrationsListResponseDataConnectedAccount`
    - Added `api_key_last_4` to `DataIntegrationsListResponseDataConnectedAccount`
    - Added enum `DataIntegrationCredentialsCredentialsType`
    - Added enum `DataIntegrationsListResponseDataAuthMethods`
    - Added enum `DataIntegrationsListResponseDataConnectedAccountAuthMethod`
    - Added service `PipesProvider`
  - **[user_management](https://workos.com/docs/reference/authkit/user)**:
    - Added model `UserInviteList`
    - Added model `UserInviteListListMetadata`
    - Made `AuthorizationCodeSessionAuthenticateRequest.client_secret` optional
    - Made `RefreshTokenSessionAuthenticateRequest.client_secret` optional
  - **[widgets](https://workos.com/docs/reference/widgets)**:
    - Added `widgets:pipes:manage` to `WidgetSessionTokenScopes`

  **Fixes**
  - **[organization_membership](https://workos.com/docs/reference/authkit/organization-membership)**:
    - Changed response of `UserManagementOrganizationMembership.list` from `UserOrganizationMembership` to `UserOrganizationMembershipList`
  - **[user_management](https://workos.com/docs/reference/authkit/user)**:
    - Changed response of `UserManagementInvitations.list` from `UserInvite` to `UserInviteList`

## [2.1.0](https://github.com/workos/workos-rust/compare/v2.0.1...v2.1.0) (2026-06-17)

- [#105](https://github.com/workos/workos-rust/pull/105) feat(generated)!: regenerate from spec (10 changes)

  **⚠️ Breaking**
  - **[api_keys](https://workos.com/docs/reference/authkit/api-keys)**:
    - Made `expires_at` required in API key models
  - **[directory_sync](https://workos.com/docs/reference/directory-sync)**:
    - Removed model `DsyncDeactivated`
    - Removed model `DsyncDeactivatedData`
    - Removed model `DsyncDeactivatedDataDomain`
    - Removed enum `DsyncDeactivatedDataType`
    - Removed enum `DsyncDeactivatedDataState`
  - **[radar](https://workos.com/docs/reference/radar)**:
    - Removed `domain_sign_up_rate_limit` from `RadarStandaloneResponseControl`
  - **[user_management](https://workos.com/docs/reference/authkit/user)**:
    - Removed `return_to` from `RevokeSession`

  **Features**
  - **[api_keys](https://workos.com/docs/reference/authkit/api-keys)**:
    - Added model `ExpireApiKey`
    - Added model `ApiKeyUpdated`
    - Added model `ApiKeyUpdatedData`
    - Added model `ApiKeyUpdatedDataOwner`
    - Added model `UserApiKeyUpdatedDataOwner`
    - Added model `ApiKeyUpdatedDataPreviousAttribute`
    - Added endpoint `POST /api_keys/{id}/expire`
  - **[audit_logs](https://workos.com/docs/reference/audit-logs)**:
    - Added `Snowflake` to `AuditLogConfigurationLogStreamType`
  - **[connect](https://workos.com/docs/reference/workos-connect/standalone)**:
    - Added `name` to `UserObject`
  - **[directory_sync](https://workos.com/docs/reference/directory-sync)**:
    - Added model `DsyncTokenCreated`
    - Added model `DsyncTokenCreatedData`
    - Added model `DsyncTokenRevoked`
    - Added model `DsyncTokenRevokedData`
  - **[user_management](https://workos.com/docs/reference/authkit/user)**:
    - Added `name` to user management models
  - **[webhooks](https://workos.com/docs/reference/webhooks)**:
    - Added `api_key.updated` to `CreateWebhookEndpointEvents`
    - Added `api_key.updated` to `UpdateWebhookEndpointEvents`

## [2.0.1](https://github.com/workos/workos-rust/compare/v2.0.0...v2.0.1) (2026-05-28)


### Bug Fixes

* **renovate:** explicitly enable minor and patch updates ([#99](https://github.com/workos/workos-rust/issues/99)) ([2639af6](https://github.com/workos/workos-rust/commit/2639af6f0749120a134dd292ef40af3df09e7fd6))
* **sdk:** omit defaulted screen_hint from auth URLs ([#102](https://github.com/workos/workos-rust/issues/102)) ([2dc6fa4](https://github.com/workos/workos-rust/commit/2dc6fa4a3bfc17a5ef399c6683c406f839f5083b))

## [2.0.0](https://github.com/workos/workos-rust/compare/v1.0.1...v2.0.0) (2026-05-26)

* [#97](https://github.com/workos/workos-rust/pull/97) feat(generated)!: regenerate from spec (8 changes)

  **⚠️ Breaking**
  * **organization_membership:** Split organization membership operations from user_management into dedicated service
    * New `OrganizationMembershipApi` service with full CRUD and role management for organization memberships
    * Moved from `UserManagementApi`: list/create/get/update/delete/deactivate/reactivate operations
    * Breaking change: symbols removed from `user_management` and moved to `organization_membership` (see compat_breaking list)
    * `Role` enum (`Single`/`Multiple` variants) moved from `user_management` to `organization_membership`
    * `client.user_management_organization_membership_groups()` removed; use `client.organization_membership().list_organization_membership_groups()` instead
    * Group membership listing now accessible via `organization_membership.list_organization_membership_groups`
  * **radar:** Remove deprecated action and control fields from Radar standalone assessment
    * Removed deprecated enum variants: `RadarStandaloneAssessRequestAction::Login`, `RadarStandaloneAssessRequestAction::Signup`, `RadarStandaloneResponseControl::CredentialStuffing`, `RadarStandaloneResponseControl::IpSignUpRateLimit`
    * Removed lenient parsing aliases for `SignUp` (`"sign up"`, `"sign_up"`) and `SignIn` (`"sign in"`, `"sign_in"`); only canonical wire values `"sign-up"` and `"sign-in"` are accepted
    * Removed fields from `RadarStandaloneAssessRequest`: `device_fingerprint` and `bot_score` (marked breaking in spec)
  * **vault:** Replaced hand-written `helpers::VaultApi` with generated `resources::VaultApi`
    * `client.vault()` now returns `resources::VaultApi` instead of `helpers::VaultApi`
    * Old custom types removed: `DataKeyPair`, `DataKey`, `KeyContext`, `ObjectMetadata`, `VaultObject`, `VaultObjectDigest`, `VaultObjectVersion`, `VaultListObjectsParams`, `VaultListObjectsResponse`, `VaultCreateDataKeyParams`, `VaultCreateObjectParams`, `VaultUpdateObjectParams`, `VaultDecryptDataKeyParams`
    * Replaced by generated types: `CreateDataKeyRequest`, `CreateDataKeyResponse`, `DecryptRequest`, `DecryptResponse`, `RekeyRequest`, `CreateObjectRequest`, `UpdateObjectRequest`, `VaultObject`, `ObjectMetadata`, `ObjectSummary`, `ObjectVersion`, `ObjectListResponse`, `VersionListResponse`
    * Local `encrypt`/`decrypt` convenience methods preserved on `resources::VaultApi` with the same behavior
    * Crypto helpers moved from `helpers::vault` to `helpers::vault_crypto` (re-exported: `VaultEncryptResult`, `local_encrypt`, `local_decrypt`, `extract_encrypted_keys`)
  * **generated:** Rename types to remove `Json` suffix and standardize naming
    * `AuditLogExportJsonState` → `AuditLogExportState`
    * `AuditLogActionJson` → `AuditLogAction`
    * `AuditLogExportJson` → `AuditLogExport`
    * `AuditLogsRetentionJson` → `AuditLogsRetention`
    * `WebhookEndpointJson` → `WebhookEndpoint`, `WebhookEndpointJsonStatus` → `WebhookEndpointStatus`
    * `RadarAction` → `RadarListAction`, `RadarType` → `RadarListType`
    * `AuditLogSchema` renamed to `AuditLogSchemaDto`; new `AuditLogSchemaInput`, `AuditLogSchemaActorInput`, `AuditLogSchemaTargetInput` types added

  **Features**
  * **vault:** Add Vault service with key management and object storage APIs
    * New `VaultApi` service providing encryption key management and encrypted object storage
    * Key management: `create_data_key`, `create_decrypt`, `create_rekey` for cryptographic operations
    * Object storage: `list_kv`, `create_kv`, `get_kv`, `get_name`, `update_kv`, `delete_kv` for managing encrypted key-value pairs
    * Metadata operations: `list_kv_metadata` and `list_kv_versions` for inspecting object history without decryption
  * **api_key:** Add expires_at field to API key models
    * New optional `expires_at` field added to `ApiKey`, `OrganizationApiKey`, `OrganizationApiKeyWithValue`, `UserApiKey`, `UserApiKeyWithValue` models
    * Allows setting expiration timestamps on API keys; null means no expiration
    * Event data models updated: `ApiKeyCreatedData` and `ApiKeyRevokedData` now include `expires_at`
    * New optional parameter `expires_at` in `CreateOrganizationApiKey` and `CreateUserApiKey` request bodies
  * **webhooks:** Add Pipes connected account events to webhook subscriptions
    * Three new webhook event types for Pipes integrations: `PIPES_CONNECTED_ACCOUNT_CONNECTED`, `PIPES_CONNECTED_ACCOUNT_DISCONNECTED`, `PIPES_CONNECTED_ACCOUNT_REAUTHORIZATION_NEEDED`
    * New model types: `PipeConnectedAccount`, `PipesConnectedAccountConnected`, `PipesConnectedAccountDisconnected`, `PipesConnectedAccountReauthorizationNeeded`
    * New enum `PipeConnectedAccountState` for connected account status tracking
  * **connect:** Add typed connect application models and new fields
    * New `ConnectApplicationM2M` and `ConnectApplicationOAuth` model types for M2M and OAuth applications
    * New fields on `ConnectApplication`: `redirect_uris`, `uses_pkce`, `is_first_party`, `was_dynamically_registered`
    * New `ConnectApplicationRedirectUri` and `ConnectApplicationOAuthRedirectUris` types
  * **generated:** Add new general-purpose models
    * New `Actor` model for representing users or API keys that performed actions
    * New `ErrorResponse` model for structured error response bodies
    * New `ListMetadata` model for cursor-based pagination metadata
    * New `VaultOrder` enum for ordering vault list results

  **Fixes**
  * **generated:** Standardize type names and fix parameter defaults in authorization service
    * Added `resource_id`, `resource_external_id`, and `resource_type_slug` filters to `ListRoleAssignmentsParams` for more granular assignment filtering
    * Added `role_slug` filter to `ListRoleAssignmentsForResourceParams` and `ListRoleAssignmentsForResourceByExternalIdParams`
    * Removed `search` parameter from `ListResourcesParams` (deprecated)
  * **connect:** Fix last_used_at field type in application credentials
    * `NewConnectApplicationSecret.last_used_at` type changed from invalid string value to ISO 8601 timestamp
    * `ApplicationCredentialsListItem.last_used_at` type changed from invalid string value to ISO 8601 timestamp
    * Ensures consistency with API contract for credential timestamp tracking
  * **sso:** Expand login_hint documentation to include custom SAML
    * Updated `GetAuthorizationUrlParams.login_hint` documentation to indicate support for custom SAML connections in addition to existing OAuth/OpenID Connect/Okta/Entra ID support

## [1.0.1](https://github.com/workos/workos-rust/compare/v1.0.0...v1.0.1) (2026-05-13)


### Bug Fixes

* **generated:** Regenerate from oagen with query encoder + typed param bodies ([e63d620](https://github.com/workos/workos-rust/commit/e63d620673b1e0b1270b1ca0ad7539d462c94e51))

## [1.0.0](https://github.com/workos/workos-rust/compare/v0.8.1...v1.0.0) (2026-05-10)


### ⚠ BREAKING CHANGES

* prep for v1

### Features

* Add ApiError, RequestOptions, and auto-paging streams ([a1a66ba](https://github.com/workos/workos-rust/commit/a1a66ba8cad6f8d79456c4f951cd00b1c0d8b108))
* **client:** Add path_segment encoder and shared auto-paging driver ([7f1c2d3](https://github.com/workos/workos-rust/commit/7f1c2d3b2490b588bf32abb8b5c76e1c0598e9e4))
* **client:** Gate retries by safety and add per-request RequestStrategy ([1fb9b1c](https://github.com/workos/workos-rust/commit/1fb9b1c8835daeb77f603b689418b0860352de6c))
* prep for v1 ([9b42e77](https://github.com/workos/workos-rust/commit/9b42e7753ae5b73ebcfb7cba9cebd9967a49c94f))
* **secret:** Add SecretString wrapper for sensitive fields ([d3729a9](https://github.com/workos/workos-rust/commit/d3729a98c9ce6c9bf654779e8c252c7887f83cd5))


### Bug Fixes

* **helpers:** Harden webhook, session, and vault crypto paths ([98b8a59](https://github.com/workos/workos-rust/commit/98b8a59781d461a78b429c4757224d86a4bb68be))

## [Unreleased]

## [1.0.0]

This release is a ground-up rebuild of the SDK. Every resource module is now
generated from the WorkOS OpenAPI spec by `oagen`; only a thin async client,
the helper layer, and the pagination/transport plumbing are hand-maintained.

### Added

- Async, builder-based `Client` with configurable timeout, retry budget, base
  URL, and pluggable transport (`reqwest` by default; `rustls-tls` and
  `native-tls` are exposed as crate features).
- Generated resource APIs covering Organizations, User Management, SSO,
  Directory Sync, Audit Logs, Authorization (FGA), Vault, Webhooks, Events,
  API Keys, Admin Portal, Connect, Feature Flags, Groups, Multi-factor Auth,
  Pipes, Radar, and Widgets.
- `RequestOptions` with `idempotency_key(...)` and `header(...)` setters; each
  generated method now has a companion `*_with_options(..., Some(&opts))`.
- Structured `ApiError` carrying `status`, `code`, `message`, `request_id`,
  `Retry-After`, full headers, and the raw response body. `Error` exposes
  `request_id()`, `code()`, `retry_after()`, plus `is_unauthorized()`,
  `is_not_found()`, `is_rate_limited()`, and `is_server_error()` predicates.
- Cursor-based auto-pagination: every list endpoint generates a
  `*_auto_paging(...)` method returning `impl futures_util::Stream`. The
  shared `auto_paginate(fetch)` helper is also re-exported for custom flows.
- Hand-maintained helper layer for AuthKit, SSO URL builders, PKCE flows,
  webhook signature verification, sealed sessions, JWKS, Vault local crypto,
  Passwordless, and a public PKCE-only client.
- Path parameters are percent-encoded as URL segments before interpolation.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` is enforced in CI;
  `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
  run on every change. Rust `1.88` (edition 2024) is pinned via
  `rust-toolchain.toml`.

### Changed

- The crate is now async-first and depends on `tokio`. Synchronous wrappers
  from earlier `0.x` releases are no longer provided.

## [0.2.0] - 2022-07-14

### Added

- Added `organization_id` to `DirectoryUser`s and `DirectoryGroup`s ([#84](https://github.com/workos/workos-rust/pull/84))

## [0.1.1] - 2022-07-11

### Changed

- Updated the endpoints used for `ChallengeFactor` and `VerifyChallenge` operations ([#81](https://github.com/workos/workos-rust/pull/81))
- Changed project status to "experimental" ([#82](https://github.com/workos/workos-rust/pull/82))

## [0.1.0] - 2022-07-01

### Added

- Initial release

[unreleased]: https://github.com/workos/workos-rust/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/workos/workos-rust/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/workos/workos-rust/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/workos/workos-rust/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/workos/workos-rust/releases/tag/66a4c78...v0.1.0
