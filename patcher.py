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
    except requests.exceptions.RequestException as e:
        print(f"Error checking tabs: {e}")
    return False

def main():
    print("Waiting for CEF tabs to become available...")
    while True:
        if find_tabs():
            print("Required tabs found, Patching!!!!")
            # Insert your code to perform actions when tabs are found
            # break
            time.sleep(0.1)
    
        else:
            print("Tabs not found, Unpatching...")
            time.sleep(0.1)

if __name__ == '__main__':
    main()
