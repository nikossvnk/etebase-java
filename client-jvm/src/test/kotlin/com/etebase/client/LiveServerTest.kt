package com.etebase.client

import java.nio.file.Files
import java.util.UUID
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test

/**
 * End-to-end tests through [OkHttpClientBridge] against a disposable, signup-enabled Etebase
 * server: `./gradlew -PjvmOnly :client-jvm:test -PetebaseServerUrl=http://127.0.0.1:3735`.
 * Skipped when no server is configured.
 */
class LiveServerTest {
    private val serverUrl: String? = System.getProperty("etebase.serverUrl")
    private val http = OkHttpClient.Builder().followRedirects(false).build()
    private lateinit var client: Client
    private lateinit var account: Account
    private lateinit var username: String

    @Before
    fun signUp() {
        assumeTrue("Set -PetebaseServerUrl to run live server tests", serverUrl != null)
        client = createClient(http, serverUrl)
        assertTrue(isEtebaseServer(client))
        username = "jvm-" + UUID.randomUUID()
        account = Account.signup(client, User(username, "$username@example.com"), PASSWORD)
    }

    @After
    fun logOut() {
        if (::account.isInitialized) account.logout()
    }

    @Test
    fun loginAndSessionRestore() {
        val restored = Account.restore(client, account.save())
        restored.fetchToken()
        assertEquals(0, restored.collectionManager().list(COL_TYPE).data.size)

        val user = User("jvm-" + UUID.randomUUID(), "login@example.com")
        Account.signup(client, user, PASSWORD).logout()
        val loggedIn = Account.login(client, user.username(), PASSWORD)
        val key = randomBytes(32u)
        val keyed = Account.restore(client, loggedIn.save(key), key)
        assertThrows(EtebaseException.Encryption::class.java) { Account.restore(client, loggedIn.save(key), randomBytes(32u)) }
        keyed.fetchToken()
        loggedIn.logout()
        assertThrows(EtebaseException.Unauthorized::class.java) { Account.login(client, user.username(), "wrong") }
    }

    @Test
    fun collectionsAndItemsRoundTrip() {
        val colMgr = account.collectionManager()
        val meta = ItemMetadata(name = "Calendar 🗓", mtime = 1_900_000_000_123L, color = "#00ff00")
        val col = colMgr.create(COL_TYPE, meta, "collection".toByteArray())
        colMgr.upload(col, FetchOptions(prefetch = PrefetchOption.AUTO))
        val other = colMgr.create(OTHER_TYPE, ItemMetadata(name = "Tasks"), ByteArray(0))
        colMgr.upload(other)

        val listed = colMgr.list(COL_TYPE)
        assertEquals(listOf(col.uid()), listed.data.map { it.uid() })
        assertTrue(listed.done)
        assertTrue(listed.removedMemberships.isEmpty())
        assertEquals(0, colMgr.list(COL_TYPE, FetchOptions(stoken = listed.stoken)).data.size)
        assertEquals(
            setOf(col.uid(), other.uid()),
            colMgr.listMulti(listOf("missing", COL_TYPE, OTHER_TYPE)).data.map { it.uid() }.toSet(),
        )
        assertEquals(0, colMgr.listMulti(listOf("missing")).data.size)
        val fetchedCol = colMgr.fetch(col.uid())
        assertEquals(meta, fetchedCol.meta())
        assertEquals(COL_TYPE, fetchedCol.collectionType())
        assertEquals(CollectionAccessLevel.ADMIN, fetchedCol.accessLevel())
        assertTrue(fetchedCol.verify())

        val itemMgr = colMgr.itemManager(col)
        val first = itemMgr.create(ItemMetadata(itemType = "event", name = "first"), "one".toByteArray())
        val second = itemMgr.create(ItemMetadata(itemType = "event", name = "second"), "two".toByteArray())
        itemMgr.batch(listOf(first, second))
        val initial = itemMgr.list()
        assertEquals(setOf(first.uid(), second.uid()), initial.data.map { it.uid() }.toSet())

        // Same Kotlin item edited and uploaded again: the saved etag must have reached it.
        first.setContent("one, edited".toByteArray())
        itemMgr.transaction(listOf(first), deps = listOf(second))
        val fetched = itemMgr.fetch(first.uid())
        assertEquals("one, edited", String(fetched.content()))
        assertEquals(first.etag(), fetched.etag())

        val updates = itemMgr.fetchUpdates(initial.data)
        assertEquals(listOf(first.uid()), updates.data.map { it.uid() })
        assertEquals(1, itemMgr.list(FetchOptions(stoken = initial.stoken)).data.size)
        val revisions = itemMgr.itemRevisions(first)
        assertEquals(listOf("one, edited", "one"), revisions.data.map { String(it.content()) })
        assertTrue(revisions.done)
        assertEquals(1, itemMgr.itemRevisions(first, FetchOptions(limit = 1u)).data.size)
    }

    @Test
    fun largeContentRoundTrips() {
        val colMgr = account.collectionManager()
        val col = colMgr.create(COL_TYPE, ItemMetadata(), ByteArray(0))
        colMgr.upload(col)
        val itemMgr = colMgr.itemManager(col)
        val content = randomBytes(1024u * 1024u)
        val item = itemMgr.create(ItemMetadata(name = "big"), content)
        itemMgr.uploadContent(item)
        itemMgr.batch(listOf(item))

        val partial = itemMgr.fetch(item.uid(), FetchOptions(prefetch = PrefetchOption.MEDIUM))
        assertTrue(partial.isMissingContent())
        assertThrows(EtebaseException.MissingContent::class.java) { partial.content() }
        itemMgr.downloadContent(partial)
        assertFalse(partial.isMissingContent())
        assertArrayEquals(content, partial.content())
        assertArrayEquals(content, itemMgr.fetch(item.uid()).content())
    }

    @Test
    fun conflictsAndTombstones() {
        val colMgr = account.collectionManager()
        val col = colMgr.create(COL_TYPE, ItemMetadata(), ByteArray(0))
        colMgr.upload(col)
        val itemMgr = colMgr.itemManager(col)
        val item = itemMgr.create(ItemMetadata(name = "shared"), "v1".toByteArray())
        itemMgr.batch(listOf(item))

        val stale = itemMgr.fetch(item.uid())
        item.setContent("v2".toByteArray())
        itemMgr.transaction(listOf(item))
        stale.setContent("stale".toByteArray())
        assertThrows(EtebaseException.Conflict::class.java) { itemMgr.transaction(listOf(stale)) }
        val staleCol = colMgr.fetch(col.uid())
        col.setContent("newer".toByteArray())
        colMgr.transaction(col)
        staleCol.setContent("stale".toByteArray())
        assertThrows(EtebaseException.Conflict::class.java) { colMgr.transaction(staleCol) }

        val listed = itemMgr.list()
        item.markDeleted()
        assertTrue(item.isDeleted())
        itemMgr.batch(listOf(item))
        val changes = itemMgr.list(FetchOptions(stoken = listed.stoken))
        assertEquals(listOf(item.uid()), changes.data.map { it.uid() })
        assertTrue(changes.data.single().isDeleted())

        col.markDeleted()
        colMgr.upload(col)
        assertTrue(colMgr.fetch(col.uid()).isDeleted())
    }

    @Test
    fun fileSystemCacheSavesAndLoads() {
        val dir = Files.createTempDirectory("etebase-cache")
        try {
            val cache = FileSystemCache.create(dir.toString(), username)
            val key = randomBytes(32u)
            cache.accountSave(account, key)
            val loaded = cache.accountLoad(client, key)
            loaded.fetchToken()

            val colMgr = loaded.collectionManager()
            val col = colMgr.create(COL_TYPE, ItemMetadata(name = "cached"), "c".toByteArray())
            colMgr.upload(col)
            val itemMgr = colMgr.itemManager(col)
            val item = itemMgr.create(ItemMetadata(name = "cached item"), "i".toByteArray())
            itemMgr.batch(listOf(item))

            assertNull(cache.loadStoken())
            cache.saveStoken("list-stoken")
            assertEquals("list-stoken", cache.loadStoken())
            cache.collectionSet(colMgr, col)
            cache.collectionSaveStoken(col.uid(), "item-stoken")
            cache.itemSetWithContent(itemMgr, col.uid(), item)
            assertEquals("item-stoken", cache.collectionLoadStoken(col.uid()))
            assertEquals(listOf(col.uid()), cache.collectionList(colMgr).map { it.uid() })
            assertEquals(col.etag(), cache.collectionGet(colMgr, col.uid()).etag())
            val cachedItem = cache.itemGet(itemMgr, col.uid(), item.uid())
            assertEquals("i", String(cachedItem.content()))
            assertEquals(listOf(item.uid()), cache.itemList(itemMgr, col.uid()).map { it.uid() })
            // A cached item keeps its saved etag: an unchanged transaction succeeds.
            cachedItem.setContent("i2".toByteArray())
            itemMgr.transaction(listOf(cachedItem))

            cache.itemUnset(itemMgr, col.uid(), item.uid())
            assertTrue(cache.itemList(itemMgr, col.uid()).isEmpty())
            cache.collectionUnset(colMgr, col.uid())
            assertTrue(cache.collectionList(colMgr).isEmpty())
            assertThrows(EtebaseException::class.java) { cache.collectionGet(colMgr, col.uid()) }
            cache.clearUserCache()
            assertThrows(EtebaseException::class.java) { cache.accountLoad(client, key) }
        } finally {
            dir.toFile().deleteRecursively()
        }
    }

    @Test
    fun invitationsAndMembers() {
        val colMgr = account.collectionManager()
        val col = colMgr.create(COL_TYPE, ItemMetadata(), ByteArray(0))
        colMgr.upload(col)
        val inviteeName = "jvm-" + UUID.randomUUID()
        val invitee = Account.signup(client, User(inviteeName, "$inviteeName@example.com"), PASSWORD)
        try {
            val invitations = account.invitationManager()
            val profile = invitations.fetchUserProfile(inviteeName)
            assertArrayEquals(invitee.invitationManager().pubkey(), profile.pubkey)
            assertNotEquals("", prettyFingerprint(profile.pubkey))
            invitations.invite(col, inviteeName, profile.pubkey, CollectionAccessLevel.READ_WRITE)
            assertEquals(1, invitations.listOutgoing().data.size)

            val incoming = invitee.invitationManager().listIncoming().data.single()
            assertEquals(col.uid(), incoming.collection())
            assertEquals(CollectionAccessLevel.READ_WRITE, incoming.accessLevel())
            assertEquals(username, incoming.senderUsername())
            invitee.invitationManager().accept(incoming)

            val members = colMgr.memberManager(col)
            assertEquals(setOf(username, inviteeName), members.list().data.map { it.username }.toSet())
            members.modifyAccessLevel(inviteeName, CollectionAccessLevel.READ_ONLY)
            assertEquals(CollectionAccessLevel.READ_ONLY, members.list().data.single { it.username == inviteeName }.accessLevel)
            members.remove(inviteeName)
            assertEquals(1, members.list().data.size)
        } finally {
            invitee.logout()
        }
    }

    private companion object {
        const val PASSWORD = "test password only"
        const val COL_TYPE = "etebase.jvm.test"
        const val OTHER_TYPE = "etebase.jvm.other"
    }
}
