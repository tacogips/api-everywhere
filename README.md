## API everywhere

Turn your google spread sheet into a json API.

see [example app](https://api-everywhere-emyxjnbuoq-an.a.run.app/?sheetUrl=
https%3A%2F%2Fdocs.google.com%2Fspreadsheets%2Fd%2F1HA4munsvl5UUlb9DKmJvhrwfGlSQ97hSQZf13M3ZO4Y%2Fedit%23gid%3D0
)

Not Production ready yet.

## Usecase

1. Use Google spreadsheet as CMS in JAM stack flow.
2. You don't wanna publish your Spreadsheet (All You need is share the sheet only your service account)
3. Run the spread sheet api server in private network(e.g. cloud run).

## how to docker build

1. make your GCP service account and enable spread API by the service account
(see https://developers.google.com/sheets/api/guides/authorizing#APIKey)

2. Put the ServiceAccount json file to `dev-secret/test-sa-key.json`

3. Docker build with `Dockerfile`. see `docker-compose.yaml`

## References Link
[(japanese)サービスアカウントで認証してGoogleSpreadsheetからデータを取得](https://dream-yt.github.io/post/spreadsheet-via-service-account/)


## TODO

- [ ] More test
- [ ] Add other API beside spread sheet (like notion?)
