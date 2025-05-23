package poc.p2p.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import poc.p2p.app.ui.theme.AppTheme

import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import uniffi.bindings_p2p.*  // <- generado por uniffi-bindgen
import java.util.concurrent.CopyOnWriteArrayList

class MainActivity : AppCompatActivity() {

    private val messages = CopyOnWriteArrayList<String>()
    private lateinit var textView: TextView
    private lateinit var input: EditText
    private lateinit var sendButton: Button

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Carga el layout base
        setContentView(buildUI())

        // Listener para recibir mensajes desde Rust
        val listener = object : EventListener {
            override fun onEvent(event: Event): String {
                runOnUiThread {
                    messages.add(event.topic + ":" + event.message)
                    textView.text = messages.joinToString("\n")
                }
                return ""
            }
        }

        // Enlaza el listener
        setListener(listener)

        // Lanza el nodo P2P
        start()

        sendButton.setOnClickListener {
            val msg = input.text.toString()
            if (msg.isNotBlank()) {
                sendMessage("chat", msg)
                input.setText("")
            }
        }
    }

    private fun buildUI(): LinearLayout {
        val layout = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(16, 16, 16, 16)
        }

        textView = TextView(this).apply {
            textSize = 16f
        }

        input = EditText(this).apply {
            hint = "Type a message..."
        }

        sendButton = Button(this).apply {
            text = "Send"
        }

        layout.addView(textView)
        layout.addView(input)
        layout.addView(sendButton)

        return layout
    }

    companion object {
        init {
            System.loadLibrary("uniffi_bindings_p2p")
        }
    }
}
