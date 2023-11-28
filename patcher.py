import requests
import time
import json

# URL to the CEF debug port
DEBUG_URL = "http://localhost:8080/json"

# Tabs to look for
TABS_TO_FIND = [
    # "MainMenu_uid7",
    # "QuickAccess_uid7",
    # "data:text/html,<body></body>",
    # "notificationtoasts_uid7",
    # "Steam Big Picture Mode",
    "SharedJSContext"
]

def find_tabs():
    """ Check if the specified tabs are available """
    try:
        response = requests.get(DEBUG_URL)
        if response.status_code == 200:
            tabs = json.loads(response.text)
            for tab in tabs:
                if tab.get('title') in TABS_TO_FIND:
                    return True
    except ConnectionError as e:
        print("Error: Could not connect to the server. Is the server running?")
        return False
    except requests.exceptions.RequestException as e:
        # print(f"Error checking tabs: {e}")
        print(f"Error checking tabs")
        # return None

    return False

def main():
    patched = False
    print("Monitoring CEF tabs...")
    while True:
        tabs_found = find_tabs()
        if tabs_found is None:
            print("Server not available, rechecking in 3 seconds...")
            time.sleep(3)
            continue
        if tabs_found and not patched:
            print("Required tabs found, patching...")
            patched = True
            # Perform patching action here
            data = {"status": "patched"}
            print(json.dumps(data))
        elif not tabs_found and patched:
            print("Tabs not found, unpatching...")
            patched = False
            # Perform unpatching action here
            data = {"status": "unpatched"}
            print(json.dumps(data))

        time.sleep(0.1)  # Delay before rechecking

if __name__ == '__main__':
    main()
