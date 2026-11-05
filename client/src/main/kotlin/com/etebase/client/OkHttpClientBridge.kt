package com.etebase.client

import okhttp3.OkHttpClient
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.RequestBody.Companion.toRequestBody

/**
 * An [HttpClient] that sends Etebase requests through [client].
 *
 * Exceptions thrown by OkHttp (e.g. `IOException`) reach Etebase as
 * [EtebaseException.Connection] with the exception's `toString()` as message.
 */
class OkHttpClientBridge(private val client: OkHttpClient) : HttpClient {
    override fun get(url: String, authToken: String?): HttpResponse =
        execute(url, authToken) { get() }

    override fun post(url: String, authToken: String?, body: ByteArray): HttpResponse =
        execute(url, authToken) { post(body.toMsgPackBody()) }

    override fun put(url: String, authToken: String?, body: ByteArray): HttpResponse =
        execute(url, authToken) { put(body.toMsgPackBody()) }

    override fun patch(url: String, authToken: String?, body: ByteArray): HttpResponse =
        execute(url, authToken) { patch(body.toMsgPackBody()) }

    override fun delete(url: String, authToken: String?): HttpResponse =
        execute(url, authToken) { delete() }

    private inline fun execute(
        url: String,
        authToken: String?,
        method: Request.Builder.() -> Request.Builder,
    ): HttpResponse {
        val request = Request.Builder()
            .url(url)
            .header("Accept", MSGPACK_TYPE)
            .apply { if (authToken != null) header("Authorization", "Token $authToken") }
            .method()
            .build()
        return client.newCall(request).execute().use { response ->
            HttpResponse(response.code.toUShort(), response.body.bytes())
        }
    }

    private companion object {
        const val MSGPACK_TYPE = "application/msgpack"
        val MSGPACK = MSGPACK_TYPE.toMediaType()

        fun ByteArray.toMsgPackBody(): RequestBody = toRequestBody(MSGPACK)
    }
}

/** Creates a [Client] for [serverUrl] (the default Etebase server when null) using [okHttpClient]. */
fun createClient(okHttpClient: OkHttpClient, serverUrl: String?): Client =
    Client(serverUrl, OkHttpClientBridge(okHttpClient))
