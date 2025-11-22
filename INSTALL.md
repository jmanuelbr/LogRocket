# Installing the Updated Log Rocket

The changes have been made to the code and a new `LogRocket.dmg` has been created. To see the changes, you need to install the updated version:

## Installation Steps

1. **Remove the old version:**
   ```bash
   rm -rf /Applications/Log\ Rocket.app
   ```

2. **Mount the new DMG:**
   ```bash
   open LogRocket.dmg
   ```

3. **Drag the app to Applications folder**
   - A Finder window will open
   - Drag "Log Rocket.app" to the Applications folder

4. **Launch the new version:**
   ```bash
   open -a "Log Rocket"
   ```

## What's Changed

- ✅ Sidebar is now **closed by default** (was open before)
- ✅ All buttons now use **icons only**:
  - 📁 (instead of "📁 Open")
  - 🔄 (instead of "🔄 Reload")
  - 🔍 (instead of "🔍 Search")
  - ⏴/⏵ (instead of "Sidebar ⏴/⏵")
  - ⬆/⬇ (instead of "Prev/Next")
- ✅ Search matches scroll to **top** instead of center

If you're still seeing the old UI after reinstalling, try:
```bash
# Force quit any running instances
killall "Log Rocket"

# Clear app cache
rm -rf ~/Library/Caches/com.jose.log-rocket

# Relaunch
open -a "Log Rocket"
```
