#import <Cocoa/Cocoa.h>
#import <WebKit/WebKit.h>

@interface Reporter : NSObject <WKScriptMessageHandler, WKNavigationDelegate>
@end
@implementation Reporter
- (void)userContentController:(WKUserContentController *)controller
      didReceiveScriptMessage:(WKScriptMessage *)message {
    if (![message.body isKindOfClass:NSString.class]) return;
    NSString *line = message.body;
    puts(line.UTF8String);
    fflush(stdout);
    if ([line hasPrefix:@"FAIL:"]) exit(1);
    if ([line isEqualToString:@"PASS"]) [NSApp terminate:nil];
}
- (void)webViewWebContentProcessDidTerminate:(WKWebView *)webView {
    fputs("WebKit content process terminated\n", stderr);
    exit(1);
}
@end

int main(int argc, char **argv) {
    @autoreleasepool {
        if (argc != 3) return 2;
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyAccessory];
        NSRect viewport = NSMakeRect(0, 0, 1280, 810);
        NSWindow *window = [[NSWindow alloc] initWithContentRect:viewport
            styleMask:NSWindowStyleMaskBorderless backing:NSBackingStoreBuffered defer:NO];
        WKWebViewConfiguration *config = [WKWebViewConfiguration new];
        if (@available(macOS 14.0, *)) {
            config.preferences.inactiveSchedulingPolicy = WKInactiveSchedulingPolicyNone;
        }
        Reporter *reporter = [Reporter new];
        [config.userContentController addScriptMessageHandler:reporter name:@"report"];
        NSString *js = [NSString stringWithContentsOfFile:@(argv[1])
            encoding:NSUTF8StringEncoding error:NULL];
        if (!js) return 2;
        [config.userContentController addUserScript:[[WKUserScript alloc] initWithSource:js
            injectionTime:WKUserScriptInjectionTimeAtDocumentStart forMainFrameOnly:YES]];
        WKWebView *web = [[WKWebView alloc] initWithFrame:viewport configuration:config];
        web.navigationDelegate = reporter;
        window.contentView = web;
        [window orderBack:nil];
        [web loadRequest:[NSURLRequest requestWithURL:[NSURL URLWithString:@(argv[2])]]];
        dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 90 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
            fputs("WebKit navigation timed out\n", stderr);
            exit(1);
        });
        [NSApp run];
    }
    return 0;
}
