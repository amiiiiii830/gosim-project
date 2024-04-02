curl -X POST 'https://github.com/login/oauth/access_token' \
     -H 'Accept: application/json' \
     -H 'Content-Type: application/json' \
     -d '{
            "client_id": "816b35c307fe6ebe19c7",
            "client_secret": "bb626e19d1aa8a0dc91bacb0e4e639691e8826e1",
            "code": "cd098119ab664248ec0c",
            "redirect_uri": "https://code.flows.network/webhook/jKRuADFii4naC7ANMFtL/register"
         }'
{"error":"bad_verification_code","error_description":"The code passed is incorrect or expired.","error_uri":"https://docs.github.com/apps/managing-oauth-apps/troubleshooting-oauth-app-access-token-request-errors/#bad-verification-code"}% 


https://github.com/login/oauth/authorize?client_id=816b35c307fe6ebe19c7&scope=read:user%20user:email&redirect_uri=https://code.flows.network/webhook/jKRuADFii4naC7ANMFtL/register&state=5cf141a504aaf8ad6dbec91485c93d7ba39452651dd70b4e5c083c13cbc9b882