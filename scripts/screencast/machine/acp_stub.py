#!/usr/bin/env python3
"""A scripted ACP agent for Sheprd's screencasts: it speaks the Agent Client
Protocol over stdio and, whatever it is asked, streams one rehearsed turn that
reads a file, edits it for real, and runs the tests. Never a real agent."""
import json, os, sys, time

# The checkout the chat runs in (a worktree of it, too), else the demo repo.
REPO = os.getcwd() if os.path.exists("forecast.py") else os.path.expanduser("~/code/forecast")
FILE = os.path.join(REPO, "forecast.py")
OLD = '''def show(days):
    for day in days:
        print(f"{day['date']}  {day['high']}°/{day['low']}°  {day['sky']}")
'''
NEW = '''def show(days, as_json=False):
    if as_json:
        print(json.dumps(days, indent=2))
        return
    for day in days:
        print(f"{day['date']}  {day['high']}°/{day['low']}°  {day['sky']}")
'''

def send(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()

def update(sid, u):
    send({"jsonrpc": "2.0", "method": "session/update",
          "params": {"sessionId": sid, "update": u}})

def say(sid, text, kind="agent_message_chunk", pace=0.035):
    for word in text.split(" "):
        update(sid, {"sessionUpdate": kind, "content": {"type": "text", "text": word + " "}})
        time.sleep(pace)

def turn(sid):
    say(sid, "Let me look at how the forecast is printed.", "agent_thought_chunk", 0.02)
    update(sid, {"sessionUpdate": "tool_call", "toolCallId": "t1", "title": "Read forecast.py",
                 "kind": "read", "status": "in_progress", "locations": [{"path": FILE}]})
    time.sleep(0.6)
    update(sid, {"sessionUpdate": "tool_call_update", "toolCallId": "t1", "status": "completed"})
    say(sid, "`show()` prints a table. I'll add an `as_json` switch and a `--json` flag.\n\n")
    source = open(FILE).read()
    update(sid, {"sessionUpdate": "tool_call", "toolCallId": "t2", "title": "Edit forecast.py",
                 "kind": "edit", "status": "in_progress", "locations": [{"path": FILE}],
                 "content": [{"type": "diff", "path": FILE, "oldText": OLD, "newText": NEW}]})
    time.sleep(0.5)
    if OLD in source:
        source = source.replace(OLD, NEW).replace(
            '    show(fetch(args.city))',
            '    show(fetch(args.city), as_json=args.json)').replace(
            '    parser.add_argument("city")',
            '    parser.add_argument("city")\n    parser.add_argument("--json", action="store_true")')
        open(FILE, "w").write(source)
    update(sid, {"sessionUpdate": "tool_call_update", "toolCallId": "t2", "status": "completed"})
    update(sid, {"sessionUpdate": "tool_call", "toolCallId": "t3", "title": "python3 -m unittest",
                 "kind": "execute", "status": "in_progress"})
    time.sleep(0.8)
    update(sid, {"sessionUpdate": "tool_call_update", "toolCallId": "t3", "status": "completed",
                 "content": [{"type": "content", "content": {"type": "text", "text": "Ran 4 tests in 0.002s\n\nOK"}}]})
    say(sid, "Done. `forecast paris --json` now prints the days as JSON, and **all 4 tests pass**.")

for line in sys.stdin:
    try:
        msg = json.loads(line)
    except ValueError:
        continue
    method, mid = msg.get("method"), msg.get("id")
    if method == "initialize":
        send({"jsonrpc": "2.0", "id": mid, "result": {"protocolVersion": 1, "agentCapabilities": {}}})
    elif method == "session/new":
        send({"jsonrpc": "2.0", "id": mid, "result": {"sessionId": "s1"}})
    elif method == "session/prompt":
        turn(msg["params"]["sessionId"])
        send({"jsonrpc": "2.0", "id": mid, "result": {"stopReason": "end_turn"}})
    elif mid is not None:
        send({"jsonrpc": "2.0", "id": mid, "error": {"code": -32601, "message": "Method not found"}})
