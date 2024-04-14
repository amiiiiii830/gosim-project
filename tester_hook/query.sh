curl -X POST 'https://github.com/login/oauth/access_token' \
     -H 'Accept: application/json' \
     -H 'Content-Type: application/json' \
     -d '{
            "client_id": "816b35c307fe6ebe19c7",
            "client_secret": "******",
            "code": "cd098119ab664248ec0c",
            "redirect_uri": "https://code.flows.network/webhook/jKRuADFii4naC7ANMFtL/register"
         }'
{"error":"bad_verification_code","error_description":"The code passed is incorrect or expired.","error_uri":"https://docs.github.com/apps/managing-oauth-apps/troubleshooting-oauth-app-access-token-request-errors/#bad-verification-code"}% 


https://github.com/login/oauth/authorize?client_id=816b35c307fe6ebe19c7&scope=read:user%20user:email&redirect_uri=https://code.flows.network/webhook/jKRuADFii4naC7ANMFtL/register&state=b7c0cb4567ab0181c1126b251ab0083319f38e43f35489dfd851caa0c813e2be