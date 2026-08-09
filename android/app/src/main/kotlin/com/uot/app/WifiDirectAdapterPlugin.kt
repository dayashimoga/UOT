package com.uot.uot_app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.wifi.p2p.*
import android.util.Log
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel

/**
 * Native Android Wi-Fi Direct P2P Adapter
 *
 * Bridges Flutter MethodChannel to Android WifiP2pManager for
 * P2P group creation, peer discovery, and connection.
 */
class WifiDirectAdapterPlugin(private val context: Context) : MethodChannel.MethodCallHandler {

    companion object {
        private const val TAG = "UOT_WIFID"
        const val CHANNEL_NAME = "com.uot.wifidirect/adapter"
        const val STATE_CHANNEL = "com.uot.wifidirect/state_stream"
        const val PEER_CHANNEL = "com.uot.wifidirect/peer_stream"
    }

    private var manager: WifiP2pManager? = null
    private var channel: WifiP2pManager.Channel? = null
    private var stateEventSink: EventChannel.EventSink? = null
    private var peerEventSink: EventChannel.EventSink? = null
    private var receiver: BroadcastReceiver? = null

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "initialize" -> handleInitialize(result)
            "createGroup" -> handleCreateGroup(call, result)
            "discoverPeers" -> handleDiscoverPeers(result)
            "connectToPeer" -> handleConnectToPeer(call, result)
            "removeGroup" -> handleRemoveGroup(result)
            else -> result.notImplemented()
        }
    }

    private fun handleInitialize(result: MethodChannel.Result) {
        try {
            manager = context.getSystemService(Context.WIFI_P2P_SERVICE) as? WifiP2pManager
            channel = manager?.initialize(context, context.mainLooper, null)

            if (manager == null || channel == null) {
                result.success(mapOf("supported" to false))
                return
            }

            // Register broadcast receiver for P2P events
            val intentFilter = IntentFilter().apply {
                addAction(WifiP2pManager.WIFI_P2P_STATE_CHANGED_ACTION)
                addAction(WifiP2pManager.WIFI_P2P_PEERS_CHANGED_ACTION)
                addAction(WifiP2pManager.WIFI_P2P_CONNECTION_CHANGED_ACTION)
                addAction(WifiP2pManager.WIFI_P2P_THIS_DEVICE_CHANGED_ACTION)
            }

            receiver = object : BroadcastReceiver() {
                override fun onReceive(ctx: Context?, intent: Intent?) {
                    when (intent?.action) {
                        WifiP2pManager.WIFI_P2P_STATE_CHANGED_ACTION -> {
                            val state = intent.getIntExtra(
                                WifiP2pManager.EXTRA_WIFI_STATE,
                                WifiP2pManager.WIFI_P2P_STATE_DISABLED
                            )
                            if (state == WifiP2pManager.WIFI_P2P_STATE_ENABLED) {
                                stateEventSink?.success("idle")
                            } else {
                                stateEventSink?.success("error")
                            }
                        }
                        WifiP2pManager.WIFI_P2P_PEERS_CHANGED_ACTION -> {
                            manager?.requestPeers(channel) { peers ->
                                peers.deviceList.forEach { device ->
                                    peerEventSink?.success(mapOf(
                                        "device_id" to device.deviceName,
                                        "device_name" to device.deviceName,
                                        "device_address" to device.deviceAddress,
                                        "is_group_owner" to device.isGroupOwner
                                    ))
                                }
                            }
                        }
                        WifiP2pManager.WIFI_P2P_CONNECTION_CHANGED_ACTION -> {
                            manager?.requestConnectionInfo(channel) { info ->
                                if (info.groupFormed) {
                                    stateEventSink?.success("connected")
                                }
                            }
                        }
                    }
                }
            }

            context.registerReceiver(receiver, intentFilter)
            Log.i(TAG, "Wi-Fi Direct initialized")
            result.success(mapOf("supported" to true))
        } catch (e: Exception) {
            Log.e(TAG, "Initialize failed: ${e.message}")
            result.success(mapOf("supported" to false, "reason" to e.message))
        }
    }

    private fun handleCreateGroup(call: MethodCall, result: MethodChannel.Result) {
        val port = call.argument<Int>("port") ?: 42000
        stateEventSink?.success("creatingGroup")

        manager?.createGroup(channel, object : WifiP2pManager.ActionListener {
            override fun onSuccess() {
                Log.i(TAG, "P2P Group created")
                // Get group info
                manager?.requestGroupInfo(channel) { group ->
                    val info = mapOf(
                        "ssid" to (group?.networkName ?: "DIRECT-UOT"),
                        "passphrase" to (group?.passphrase ?: ""),
                        "frequency_mhz" to 5180,
                        "group_owner_ip" to "192.168.49.1",
                        "port" to port,
                        "is_group_owner" to true
                    )
                    stateEventSink?.success("groupCreated")
                    result.success(info)
                }
            }

            override fun onFailure(reason: Int) {
                Log.e(TAG, "Create group failed: $reason")
                stateEventSink?.success("error")
                result.success(null)
            }
        })
    }

    private fun handleDiscoverPeers(result: MethodChannel.Result) {
        stateEventSink?.success("discoveringPeers")
        manager?.discoverPeers(channel, object : WifiP2pManager.ActionListener {
            override fun onSuccess() {
                Log.i(TAG, "Peer discovery started")
                result.success(true)
            }

            override fun onFailure(reason: Int) {
                Log.e(TAG, "Peer discovery failed: $reason")
                result.success(false)
            }
        })
    }

    private fun handleConnectToPeer(call: MethodCall, result: MethodChannel.Result) {
        val address = call.argument<String>("deviceAddress") ?: return result.success(false)

        val config = WifiP2pConfig().apply {
            deviceAddress = address
            groupOwnerIntent = 0 // Prefer to be client
        }

        stateEventSink?.success("connecting")
        manager?.connect(channel, config, object : WifiP2pManager.ActionListener {
            override fun onSuccess() {
                Log.i(TAG, "Connection initiated to $address")
                result.success(true)
            }

            override fun onFailure(reason: Int) {
                Log.e(TAG, "Connection failed: $reason")
                stateEventSink?.success("error")
                result.success(false)
            }
        })
    }

    private fun handleRemoveGroup(result: MethodChannel.Result) {
        manager?.removeGroup(channel, object : WifiP2pManager.ActionListener {
            override fun onSuccess() {
                stateEventSink?.success("idle")
                result.success(true)
            }

            override fun onFailure(reason: Int) {
                result.success(false)
            }
        })
    }

    fun createStateStreamHandler(): EventChannel.StreamHandler {
        return object : EventChannel.StreamHandler {
            override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                stateEventSink = events
            }
            override fun onCancel(arguments: Any?) {
                stateEventSink = null
            }
        }
    }

    fun createPeerStreamHandler(): EventChannel.StreamHandler {
        return object : EventChannel.StreamHandler {
            override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                peerEventSink = events
            }
            override fun onCancel(arguments: Any?) {
                peerEventSink = null
            }
        }
    }

    fun dispose() {
        try {
            context.unregisterReceiver(receiver)
        } catch (_: Exception) {}
        manager?.removeGroup(channel, null)
    }
}
