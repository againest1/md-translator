# Translation preview

This document checks the complete translation and native preview workflow.

## Features

- Translate the whole document with the provider configured in Zed.
- Keep the original file unchanged.
- Reopen cached translations without another API request.

| Item | Behavior |
| --- | --- |
| Code | Preserved exactly |
| Preview | Rendered inside Zed |

Use `md-translator` to translate Markdown. Read [the features](#features) again.

```javascript
// Code must stay unchanged.
console.log("Hello, world!");
```
