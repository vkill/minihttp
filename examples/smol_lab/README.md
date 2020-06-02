### Dev

```
wscat --connect ws://127.0.0.1:8080/ws
```

```
open http://127.0.0.1:8080/


var source = new EventSource("http://127.0.0.1:8080/sse");
source.onopen = function (event) {
  console.log("onopen")
};
source.onmessage = function (event) {
  var data = event.data;
  console.log("onmessage", data)
};
```
