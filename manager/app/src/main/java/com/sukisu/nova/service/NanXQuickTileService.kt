package com.sukisu.nova.service

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import androidx.annotation.RequiresApi
import com.sukisu.nova.ui.util.KsuCliKt

@RequiresApi(Build.VERSION_CODES.N)
class NanXQuickTileService : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        updateTileState()
    }

    override fun onClick() {
        super.onClick()
        val tile = qsTile ?: return
        val isActive = tile.state == Tile.STATE_ACTIVE
        if (isActive) {
            tile.state = Tile.STATE_INACTIVE
            tile.label = "NanX-SU: Safe"
        } else {
            tile.state = Tile.STATE_ACTIVE
            tile.label = "NanX-SU: Active"
        }
        tile.updateTile()
    }

    private fun updateTileState() {
        val tile = qsTile ?: return
        val rootActive = KsuCliKt.rootAvailable()
        tile.state = if (rootActive) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        tile.label = if (rootActive) "NanX-SU: Active" else "NanX-SU: Standby"
        tile.updateTile()
    }
}
