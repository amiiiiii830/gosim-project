## Project Data for Frontend Consumption

Get data from DB:

List issues:
```
GET https://code.flows.network/webhook/5xAx1Yru2BqZ9PUacb3q/issues?page=1&page_size=2
```

Write data to DB:

Assign budget to an issue:
```
POST https://code.flows.network/webhook/5xAx1Yru2BqZ9PUacb3q/budget
{
    "issue_id": "https://github.com/adavijit-navsoft/hacktoberfest-easy/issues/30",
    "issue_budget": 100
}

```


Final approval for an issue, OK to send money:
```
POST https://code.flows.network/webhook/5xAx1Yru2BqZ9PUacb3q/conclude
{
    "issue_id": "https://github.com/ajjayymahato/HelloHacktoberfest/issues/6",
    "issue_budget_approved": true
}
```



