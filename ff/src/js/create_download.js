Components.utils.import("resource://gre/modules/Downloads.jsm");

return Downloads.createDownload({
	'source': arguments[0],
	'target': arguments[1],
}).then(function(d) {
	return d.start();
});
