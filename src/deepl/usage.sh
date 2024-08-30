# Script to get the usage of the API
curl -X GET 'https://api-free.deepl.com/v2/usage' \
--header "Authorization: DeepL-Auth-Key $DEEPL_API_KEY"