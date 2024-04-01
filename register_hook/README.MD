## User Regsiteration Tool

GitHub OAuth is used to authenticate users. 

Embed the following url in registration page:
```
https://github.com/login/oauth/authorize?client_id=816b35c307fe6ebe19c7&scope=read:user%20user:email&redirect_uri=https://code.flows.network/webhook/jKRuADFii4naC7ANMFtL/register&state=9fb5be81d17be7ab04b5c77d7bbd754d7b1f6e8a5844775356d56e189bbdda54
```


User is directed to OAuth page

![OAuth](GitHub-OAuth.png)



GitHub sends one-time code to the redirect_uri
```
https://code.flows.network/webhook/jKRuADFii4naC7ANMFtL/register?code=1b2e3f4g5h6i7j8k9l0m
```

The webhook flows function exchanges the code for an access token and retrieves user information.

With user's token, the function retrieves his profile info and writes a participant record in database.

## issue

1. high failure rate to get token

```
3/31/2024, 3:41:32 PM

{ "level": "ERROR", "message": "failed to get token", "target": "the_register_hook" }

3/31/2024, 3:41:32 PM

{ "level": "ERROR", "message": "Exchange token Response: \"{\\\"error\\\":\\\"bad_verification_code\\\",\\\"error_description\\\":\\\"The code passed is incorrect or expired.\\\",\\\"error_uri\\\":\\\"https://docs.github.com/apps/managing-oauth-apps/troubleshooting-oauth-app-access-token-request-errors/#bad-verification-code\\\"}\"", "target": "the_register_hook" }

3/31/2024, 3:41:38 PM

{ "level": "ERROR", "message": "failed to get token", "target": "the_register_hook" }

3/31/2024, 3:41:38 PM

{ "level": "ERROR", "message": "Exchange token Response: \"{\\\"error\\\":\\\"bad_verification_code\\\",\\\"error_description\\\":\\\"The code passed is incorrect or expired.\\\",\\\"error_uri\\\":\\\"https://docs.github.com/apps/managing-oauth-apps/troubleshooting-oauth-app-access-token-request-errors/#bad-verification-code\\\"}\"", "target": "the_register_hook" }

3/31/2024, 3:41:30 PM

{ "level": "INFO", "message": "Token: \"placeholder\"", "target": "the_register_hook" }

```

2. Front-end needs a verification code generator, gen-verifier.ts is deno runnable for dev.
