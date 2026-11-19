package com.sukisu.nova.ui.screen.terminal

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.sukisu.nova.ui.util.execRootCmd

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TerminalScreen(
    onBackClick: () -> Unit
) {
    var commandInput by remember { mutableStateOf("") }
    val logs = remember { mutableStateListOf("NanX Shell Root Terminal v1.0", "Type 'help' or any command...") }

    fun executeCommand() {
        if (commandInput.isBlank()) return
        val cmd = commandInput.trim()
        logs.add("# $cmd")
        commandInput = ""

        val output = execRootCmd(cmd)
        if (output.isNotBlank()) {
            logs.add(output)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("NanX Root Shell") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .background(Color(0xFF121212))
        ) {
            LazyColumn(
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth()
                    .padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                items(logs) { line ->
                    Text(
                        text = line,
                        color = if (line.startsWith("#")) Color(0xFF4CAF50) else Color(0xFFE0E0E0),
                        fontFamily = FontFamily.Monospace,
                        fontSize = 13.sp
                    )
                }
            }

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                OutlinedTextField(
                    value = commandInput,
                    onValueChange = { commandInput = it },
                    modifier = Modifier.weight(1f),
                    placeholder = { Text("Enter root command...", color = Color.Gray) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                    keyboardActions = KeyboardActions(onSend = { executeCommand() }),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedTextColor = Color.White,
                        unfocusedTextColor = Color.White
                    )
                )

                Spacer(modifier = Modifier.width(8.dp))

                IconButton(onClick = { executeCommand() }) {
                    Icon(
                        Icons.AutoMirrored.Filled.Send,
                        contentDescription = "Run",
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
            }
        }
    }
}
