/*
 * Copyright 2021, Collabora, Ltd.
 *
 * SPDX-License-Identifier: BSL-1.0
 */
package org.khronos.spdx.dual_license_finder

import com.github.ajalt.clikt.core.CliktCommand
import com.github.ajalt.clikt.parameters.arguments.argument
import com.github.ajalt.clikt.parameters.arguments.multiple
import com.github.ajalt.clikt.parameters.types.file
import org.spdx.library.ModelCopyManager
import org.spdx.library.SpdxConstants
import org.spdx.library.model.SpdxDocument
import org.spdx.library.model.SpdxFile
import org.spdx.storage.ISerializableModelStore
import org.spdx.storage.simple.InMemSpdxStore
import org.spdx.tagvaluestore.TagValueStore
import java.io.File


class Scanner : CliktCommand() {
    private val files: List<File> by argument(help = "Path to an SPDX Tag/Value data file like produced by reuse spdx")
            .file(mustExist = true)
            .multiple(required = true)

    private val predicate: IFilePredicate = KhronosDualLicensePredicate()

    private val copyManager: ModelCopyManager = ModelCopyManager()
    private val store: ISerializableModelStore = TagValueStore(InMemSpdxStore())

    data class ParsedDoc(val documentUri: String, val spdxDocument: SpdxDocument)

    private fun parseDoc(infile: File): ParsedDoc {
        val documentUri = infile.inputStream().use {
            return@use store.deSerialize(it, false)
        }
        val doc = SpdxDocument(store, documentUri, copyManager, false)
        doc.verify().let {
            if (it.isNotEmpty()) {
                println("Failed verification: $it")
                throw RuntimeException("Verification error")
            }
        }
        return ParsedDoc(documentUri, doc)
    }

    private fun process(infile: File) {
        with(parseDoc(infile)) {
            spdxDocument.documentDescribes
                    .filter { it.type == SpdxConstants.CLASS_SPDX_FILE }
                    .map { it as SpdxFile }
                    .filter { predicate.matches(it) }
                    .forEach {
                        println(it.name.get().replace("\\", "/"))
                    }
        }
    }

    /**
     * Perform actions after parsing is complete and this command is invoked.
     *
     * This is called after command line parsing is complete. If this command is a subcommand, this will only
     * be called if the subcommand is invoked.
     *
     * If one of this command's subcommands is invoked, this is called before the subcommand's arguments are
     * parsed.
     */
    override fun run() {
        // if we're given more than one spdx file, our headings list the spdx file name
        val showFilename = files.size > 1
        if (!showFilename)
            println("Showing files ${predicate.describe()}:")
        files.forEach {
            if (showFilename)
                println("Showing files described in ${it.name} that are ${predicate.describe()}:")
            process(it)
        }
    }

}

fun main(args: Array<String>) = Scanner().main(args)
