package com.sukisu.nova.ui.screen.settings.mount

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.sukisu.nova.R

enum class MountEngine(val key: String, val title: String, val description: String) {
    OVERLAY_FS("overlayfs", "OverlayFS (Default)", "Standard system overlayfs mounts"),
    NO_MOUNT("nomount", "NoMount LKM Metamodule", "Kernel VFS path redirection without mount entries"),
    IMAGE_EROFS("image_erofs", "EROFS Image Container", "Single compressed EROFS container with whiteout 0:0"),
    IMAGE_EXT4("image_ext4", "EXT4 Image Container", "Read-write EXT4 container image");

    companion object {
        fun fromKey(key: String): MountEngine {
            return entries.find { it.key == key } ?: OVERLAY_FS
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MountEngineScreen(
    currentEngine: String,
    autoRebuild: Boolean,
    loopMasking: Boolean,
    onEngineSelected: (String) -> Unit,
    onAutoRebuildToggled: (Boolean) -> Unit,
    onLoopMaskingToggled: (Boolean) -> Unit,
    onRebuildImageRequested: () -> Unit,
    onBackClick: () -> Unit
) {
    var isRebuilding by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.mount_engine_title)) },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            item {
                Text(
                    text = "Mount Engine & Container Metamodule",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.primary
                )
                Text(
                    text = "Choose how system-modifying modules are applied to the filesystem. Image containers package all system files into a single disk image with support for opaque and whiteout (0:0) features.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            // Engine choices
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            text = "Active Mount Engine",
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.height(8.dp))

                        MountEngine.entries.forEach { engine ->
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(vertical = 4.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                RadioButton(
                                    selected = (currentEngine == engine.key),
                                    onClick = { onEngineSelected(engine.key) }
                                )
                                Column(modifier = Modifier.padding(start = 8.dp)) {
                                    Text(text = engine.title, style = MaterialTheme.typography.bodyLarge)
                                    Text(
                                        text = engine.description,
                                        style = MaterialTheme.typography.bodySmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant
                                    )
                                }
                            }
                        }
                    }
                }
            }

            // Options
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            text = "Container Options",
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.height(8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("Auto-rebuild Image", style = MaterialTheme.typography.bodyLarge)
                                Text(
                                    "Rebuild container image automatically when modules are installed or removed",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Switch(checked = autoRebuild, onCheckedChange = onAutoRebuildToggled)
                        }

                        Divider(modifier = Modifier.padding(vertical = 8.dp))

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text("Loop Device Stealth Masking", style = MaterialTheme.typography.bodyLarge)
                                Text(
                                    "Hide loop device attributes from detection scripts",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                            }
                            Switch(checked = loopMasking, onCheckedChange = onLoopMaskingToggled)
                        }
                    }
                }
            }

            // Action
            item {
                Button(
                    onClick = {
                        isRebuilding = true
                        onRebuildImageRequested()
                        isRebuilding = false
                    },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isRebuilding
                ) {
                    Icon(Icons.Default.Refresh, contentDescription = null)
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(if (isRebuilding) "Rebuilding Image..." else "Rebuild Container Image Now")
                }
            }
        }
    }
}
