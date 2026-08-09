package com.uot.uot_app

import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.os.ParcelUuid
import android.util.Log
import io.flutter.plugin.common.EventChannel
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.UUID

/**
 * Native Android BLE GATT Adapter
 *
 * Bridges Flutter MethodChannel to Android BluetoothLeAdvertiser and
 * BluetoothGattServer for UOT BLE discovery and data transfer.
 */
class BleAdapterPlugin(private val context: Context) : MethodChannel.MethodCallHandler {

    companion object {
        private const val TAG = "UOT_BLE"
        const val CHANNEL_NAME = "com.uot.ble/adapter"
        const val STATE_CHANNEL = "com.uot.ble/state_stream"
        const val SCAN_CHANNEL = "com.uot.ble/scan_stream"
    }

    private var bluetoothManager: BluetoothManager? = null
    private var bluetoothAdapter: BluetoothAdapter? = null
    private var advertiser: BluetoothLeAdvertiser? = null
    private var scanner: BluetoothLeScanner? = null
    private var gattServer: BluetoothGattServer? = null
    private var stateEventSink: EventChannel.EventSink? = null
    private var scanEventSink: EventChannel.EventSink? = null
    private var serviceUuid: UUID? = null

    override fun onMethodCall(call: MethodCall, result: MethodChannel.Result) {
        when (call.method) {
            "initialize" -> handleInitialize(call, result)
            "startAdvertising" -> handleStartAdvertising(call, result)
            "stopAdvertising" -> handleStopAdvertising(result)
            "startScanning" -> handleStartScanning(call, result)
            "stopScanning" -> handleStopScanning(result)
            "sendData" -> handleSendData(call, result)
            else -> result.notImplemented()
        }
    }

    private fun handleInitialize(call: MethodCall, result: MethodChannel.Result) {
        try {
            bluetoothManager = context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
            bluetoothAdapter = bluetoothManager?.adapter

            if (bluetoothAdapter == null || !bluetoothAdapter!!.isEnabled) {
                result.success(mapOf("supported" to false, "reason" to "Bluetooth not available"))
                return
            }

            advertiser = bluetoothAdapter!!.bluetoothLeAdvertiser
            scanner = bluetoothAdapter!!.bluetoothLeScanner
            serviceUuid = UUID.fromString(call.argument<String>("serviceUuid") ?: return)

            // Create GATT server
            gattServer = bluetoothManager!!.openGattServer(context, gattServerCallback)

            // Add UOT service
            val service = BluetoothGattService(serviceUuid, BluetoothGattService.SERVICE_TYPE_PRIMARY)

            val controlChar = BluetoothGattCharacteristic(
                UUID.fromString(call.argument<String>("charControlUuid")),
                BluetoothGattCharacteristic.PROPERTY_WRITE or BluetoothGattCharacteristic.PROPERTY_NOTIFY,
                BluetoothGattCharacteristic.PERMISSION_WRITE or BluetoothGattCharacteristic.PERMISSION_READ
            )
            val dataChar = BluetoothGattCharacteristic(
                UUID.fromString(call.argument<String>("charDataUuid")),
                BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE or BluetoothGattCharacteristic.PROPERTY_NOTIFY,
                BluetoothGattCharacteristic.PERMISSION_WRITE or BluetoothGattCharacteristic.PERMISSION_READ
            )

            service.addCharacteristic(controlChar)
            service.addCharacteristic(dataChar)
            gattServer?.addService(service)

            Log.i(TAG, "BLE GATT Server initialized with service $serviceUuid")
            result.success(mapOf("supported" to true))
            stateEventSink?.success("poweredOn")
        } catch (e: Exception) {
            Log.e(TAG, "Initialize failed: ${e.message}")
            result.success(mapOf("supported" to false, "reason" to e.message))
        }
    }

    private fun handleStartAdvertising(call: MethodCall, result: MethodChannel.Result) {
        if (advertiser == null) {
            result.success(false)
            return
        }
        try {
            val settings = AdvertiseSettings.Builder()
                .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
                .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_HIGH)
                .setConnectable(true)
                .setTimeout(0)
                .build()

            val data = AdvertiseData.Builder()
                .setIncludeDeviceName(true)
                .addServiceUuid(ParcelUuid(serviceUuid))
                .build()

            val scanResponse = AdvertiseData.Builder()
                .setIncludeDeviceName(false)
                .addServiceData(
                    ParcelUuid(serviceUuid),
                    (call.argument<String>("payload") ?: "{}").toByteArray(Charsets.UTF_8)
                )
                .build()

            advertiser!!.startAdvertising(settings, data, scanResponse, advertiseCallback)
            result.success(true)
        } catch (e: Exception) {
            Log.e(TAG, "startAdvertising failed: ${e.message}")
            result.success(false)
        }
    }

    private fun handleStopAdvertising(result: MethodChannel.Result) {
        try {
            advertiser?.stopAdvertising(advertiseCallback)
            stateEventSink?.success("poweredOn")
            result.success(true)
        } catch (e: Exception) {
            result.success(false)
        }
    }

    private fun handleStartScanning(call: MethodCall, result: MethodChannel.Result) {
        if (scanner == null) {
            result.success(false)
            return
        }
        try {
            val filters = listOf(
                ScanFilter.Builder()
                    .setServiceUuid(ParcelUuid(serviceUuid))
                    .build()
            )
            val settings = ScanSettings.Builder()
                .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                .build()

            scanner!!.startScan(filters, settings, scanCallback)
            stateEventSink?.success("scanning")
            result.success(true)
        } catch (e: Exception) {
            Log.e(TAG, "startScanning failed: ${e.message}")
            result.success(false)
        }
    }

    private fun handleStopScanning(result: MethodChannel.Result) {
        try {
            scanner?.stopScan(scanCallback)
            stateEventSink?.success("poweredOn")
            result.success(true)
        } catch (e: Exception) {
            result.success(false)
        }
    }

    private fun handleSendData(call: MethodCall, result: MethodChannel.Result) {
        // TODO: implement GATT write to connected device
        result.success(false)
    }

    // Callbacks
    private val advertiseCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(settingsInEffect: AdvertiseSettings?) {
            Log.i(TAG, "Advertising started")
            stateEventSink?.success("advertising")
        }

        override fun onStartFailure(errorCode: Int) {
            Log.e(TAG, "Advertising failed: $errorCode")
            stateEventSink?.success("poweredOn")
        }
    }

    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult?) {
            result?.device?.let { device ->
                val payload = result.scanRecord?.getServiceData(ParcelUuid(serviceUuid))
                val payloadMap = if (payload != null) {
                    try {
                        val json = String(payload, Charsets.UTF_8)
                        org.json.JSONObject(json).let { obj ->
                            mapOf(
                                "device_name" to obj.optString("device_name", ""),
                                "device_hash" to obj.optString("device_hash", ""),
                                "wifi_ip" to obj.optString("wifi_ip", null),
                                "port" to obj.optInt("port", 42000)
                            )
                        }
                    } catch (_: Exception) { null }
                } else null

                scanEventSink?.success(mapOf(
                    "id" to device.address,
                    "name" to (device.name ?: "Unknown"),
                    "rssi" to result.rssi,
                    "payload" to payloadMap
                ))
            }
        }
    }

    private val gattServerCallback = object : BluetoothGattServerCallback() {
        override fun onConnectionStateChange(device: BluetoothDevice?, status: Int, newState: Int) {
            when (newState) {
                BluetoothProfile.STATE_CONNECTED -> {
                    Log.i(TAG, "GATT client connected: ${device?.address}")
                    stateEventSink?.success("connected")
                }
                BluetoothProfile.STATE_DISCONNECTED -> {
                    Log.i(TAG, "GATT client disconnected: ${device?.address}")
                    stateEventSink?.success("poweredOn")
                }
            }
        }

        override fun onCharacteristicWriteRequest(
            device: BluetoothDevice?,
            requestId: Int,
            characteristic: BluetoothGattCharacteristic?,
            preparedWrite: Boolean,
            responseNeeded: Boolean,
            offset: Int,
            value: ByteArray?
        ) {
            Log.d(TAG, "GATT write: ${value?.size} bytes on ${characteristic?.uuid}")
            if (responseNeeded) {
                gattServer?.sendResponse(device, requestId, BluetoothGatt.GATT_SUCCESS, offset, value)
            }
        }
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

    fun createScanStreamHandler(): EventChannel.StreamHandler {
        return object : EventChannel.StreamHandler {
            override fun onListen(arguments: Any?, events: EventChannel.EventSink?) {
                scanEventSink = events
            }
            override fun onCancel(arguments: Any?) {
                scanEventSink = null
            }
        }
    }

    fun dispose() {
        advertiser?.stopAdvertising(advertiseCallback)
        scanner?.stopScan(scanCallback)
        gattServer?.close()
    }
}
