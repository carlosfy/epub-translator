# Get the list of languages supported by DeepL
curl -X GET 'https://api-free.deepl.com/v2/languages?type=target' \
--header "Authorization: DeepL-Auth-Key $DEEPL_API_KEY"