/* Moves the pointer to x, y (in points) inside the machine, as a hand would
 * before clicking, so the app under it redraws its hover first. Sheprd
 * Agent's own click moves and presses 8 ms apart, which a busy machine
 * does not redraw in, and the press then lands on what was under the old
 * position. Runs through `sheprd machine exec`, so it acts with the
 * agent's Accessibility permission.
 *   cc -O2 -o hover hover.c -framework ApplicationServices */
#include <ApplicationServices/ApplicationServices.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc != 3) return 2;
    CGPoint point = CGPointMake(atof(argv[1]), atof(argv[2]));
    CGEventSourceRef source = CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
    CGEventRef move = CGEventCreateMouseEvent(source, kCGEventMouseMoved, point, kCGMouseButtonLeft);
    if (!move) return 1;
    CGEventPost(kCGHIDEventTap, move);
    CFRelease(move);
    if (source) CFRelease(source);
    return 0;
}
