package com.uot.uot_app

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var blePlugin: BleAdapterPlugin? = null
    private var wifiDirectPlugin: WifiDirectAdapterPlugin? = null

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        try {
            // Register BLE Adapter Plugin
            val ble = BleAdapterPlugin(context)
            blePlugin = ble
            MethodChannel(flutterEngine.dartExecutor.binaryMessenger, BleAdapterPlugin.CHANNEL_NAME)
                .setMethodCallHandler(ble)
            EventChannel(flutterEngine.dartExecutor.binaryMessenger, BleAdapterPlugin.STATE_CHANNEL)
                .setStreamHandler(ble.createStateStreamHandler())
            EventChannel(flutterEngine.dartExecutor.binaryMessenger, BleAdapterPlugin.SCAN_CHANNEL)
                .setStreamHandler(ble.createScanStreamHandler())

            // Register Wi-Fi Direct Plugin
            val wifi = WifiDirectAdapterPlugin(context)
            wifiDirectPlugin = wifi
            MethodChannel(flutterEngine.dartExecutor.binaryMessenger, WifiDirectAdapterPlugin.CHANNEL_NAME)
                .setMethodCallHandler(wifi)
            EventChannel(flutterEngine.dartExecutor.binaryMessenger, WifiDirectAdapterPlugin.STATE_CHANNEL)
                .setStreamHandler(wifi.createStateStreamHandler())
            EventChannel(flutterEngine.dartExecutor.binaryMessenger, WifiDirectAdapterPlugin.PEER_CHANNEL)
                .setStreamHandler(wifi.createPeerStreamHandler())
        } catch (e: Exception) {
            android.util.Log.e("UOT_MAIN", "Error configuring plugins: ${e.message}")
        }
    }

    override fun onDestroy() {
        blePlugin?.dispose()
        wifiDirectPlugin?.dispose()
        super.onDestroy()
    }
}
