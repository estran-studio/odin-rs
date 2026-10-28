cmake_minimum_required(VERSION 3.22)
project(OdinFidelityReference VERSION 2.3.4 LANGUAGES C CXX ASM)
set(ODIN_UPSTREAM "" CACHE PATH "Dépôt Odin 2 amont complet, en lecture seule")
set(ODIN_JUCE "" CACHE PATH "JUCE externe au commit gitlink amont; vide = CMake amont")
set(ODIN_TUNING "" CACHE PATH "tuning-library externe; vide = sous-module amont")
set(ODIN_JSON "" CACHE PATH "json externe; vide = sous-module amont")
option(ODIN_AUDIO_MODULES_ONLY "Compiler les modules audio sans processeur ni dépendances tuning/json" OFF)
if(NOT EXISTS "${ODIN_UPSTREAM}/Source/audio/ADSR.cpp")
    message(FATAL_ERROR "ODIN_UPSTREAM doit désigner les vraies sources Odin 2")
endif()
# Aucun téléchargement, aucune modification des sous-modules ou des sources amont.
if(ODIN_JUCE)
    add_subdirectory("${ODIN_JUCE}" "${CMAKE_BINARY_DIR}/juce" EXCLUDE_FROM_ALL)
    # Tous les vrais modules audio, sans prendre le projet plugin CLAP/LV2.
    juce_add_console_app(odin_audio_modules)
    file(GLOB_RECURSE AUDIO_SOURCES CONFIGURE_DEPENDS "${ODIN_UPSTREAM}/Source/audio/*.cpp")
    juce_add_binary_data(OdinModuleAssets SOURCES "${ODIN_UPSTREAM}/assets/font/aldrich_regular.ttf")
    target_sources(odin_audio_modules PRIVATE audio_modules.cpp ${AUDIO_SOURCES})
    target_compile_features(odin_audio_modules PRIVATE cxx_std_17)
    target_link_libraries(odin_audio_modules PRIVATE OdinModuleAssets
        juce::juce_audio_processors juce::juce_dsp juce::juce_audio_formats)
    target_include_directories(odin_audio_modules PRIVATE "${ODIN_UPSTREAM}/Source" "${CMAKE_BINARY_DIR}/include")
    target_compile_definitions(odin_audio_modules PRIVATE
        JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_MODAL_LOOPS_PERMITTED=1
        CMAKE_SOURCE_DIRECTORY="${ODIN_UPSTREAM}")
    juce_generate_juce_header(odin_audio_modules)
    file(MAKE_DIRECTORY "${CMAKE_BINARY_DIR}/include")
    file(WRITE "${CMAKE_BINARY_DIR}/include/GitCommitId.h" "#define GIT_COMMIT_ID \"reference-external-deps\"\n")
    if(ODIN_AUDIO_MODULES_ONLY)
        return()
    endif()
    if(NOT ODIN_TUNING)
        set(ODIN_TUNING "${ODIN_UPSTREAM}/libs/tuning-library")
    endif()
    if(NOT ODIN_JSON)
        set(ODIN_JSON "${ODIN_UPSTREAM}/libs/json")
    endif()
    juce_add_plugin(OdinReferenceCore
        VERSION 2.4.1 COMPANY_NAME "TheWaveWarden" PLUGIN_NAME "Odin2"
        PLUGIN_MANUFACTURER_CODE WAWA PLUGIN_CODE ODIN IS_SYNTH TRUE
        NEEDS_MIDI_INPUT TRUE COPY_PLUGIN_AFTER_BUILD FALSE FORMATS Standalone)
    # Reprendre exactement la liste des unités du CMake amont: les fichiers
    # PluginProcessor*.cpp inclus textuellement ne doivent pas être compilés deux fois.
    file(STRINGS "${ODIN_UPSTREAM}/CMakeLists.txt" UPSTREAM_CPP_LINES REGEX "\"Source/.*\\.cpp\"")
    foreach(LINE IN LISTS UPSTREAM_CPP_LINES)
        string(REGEX MATCH "Source/[^\"]+\\.cpp" RELATIVE_SOURCE "${LINE}")
        target_sources(OdinReferenceCore PRIVATE "${ODIN_UPSTREAM}/${RELATIVE_SOURCE}")
    endforeach()
    add_subdirectory("${ODIN_UPSTREAM}/assets" "${CMAKE_BINARY_DIR}/assets" EXCLUDE_FROM_ALL)
    target_link_libraries(OdinReferenceCore PUBLIC Odin2_BinaryData
        juce::juce_audio_basics juce::juce_audio_devices juce::juce_audio_formats
        juce::juce_audio_plugin_client juce::juce_audio_processors juce::juce_audio_utils
        juce::juce_core juce::juce_cryptography juce::juce_data_structures juce::juce_dsp
        juce::juce_events juce::juce_graphics juce::juce_gui_basics juce::juce_gui_extra juce::juce_opengl)
    target_include_directories(OdinReferenceCore PUBLIC
        "${ODIN_UPSTREAM}/Source" "${ODIN_TUNING}" "${ODIN_JSON}/include" "${CMAKE_BINARY_DIR}/include")
    target_compile_definitions(OdinReferenceCore PUBLIC
        JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_MODAL_LOOPS_PERMITTED=1
        JUCE_DISPLAY_SPLASH_SCREEN=0 JUCE_VST3_CAN_REPLACE_VST2=0 JUCE_STRICT_REFCOUNTEDPOINTER=1
        CMAKE_SOURCE_DIRECTORY="${ODIN_UPSTREAM}")
    target_compile_features(OdinReferenceCore PRIVATE cxx_std_17)
    juce_generate_juce_header(OdinReferenceCore)
    set(REFERENCE_CORE OdinReferenceCore)
    set(REFERENCE_GENERATED_INCLUDE "${CMAKE_BINARY_DIR}/include")
else()
    set(ODIN2_COPY_PLUGIN_AFTER_BUILD OFF CACHE BOOL "" FORCE)
    add_subdirectory("${ODIN_UPSTREAM}" "${CMAKE_BINARY_DIR}/upstream" EXCLUDE_FROM_ALL)
    set(REFERENCE_CORE Odin2)
    set(REFERENCE_GENERATED_INCLUDE "${CMAKE_BINARY_DIR}/upstream/include")
endif()
add_executable(odin_reference reference.cpp)
target_compile_features(odin_reference PRIVATE cxx_std_17)
target_include_directories(odin_reference PRIVATE
    "${ODIN_UPSTREAM}/Source" "${ODIN_UPSTREAM}/libs/tuning-library"
    "${ODIN_UPSTREAM}/libs/json/include" "${REFERENCE_GENERATED_INCLUDE}"
    "$<TARGET_PROPERTY:${REFERENCE_CORE},JUCE_GENERATED_SOURCES_DIRECTORY>"
)
target_link_libraries(odin_reference PRIVATE ${REFERENCE_CORE})
if(APPLE)
    target_compile_definitions(odin_reference PRIVATE ODIN_MAC=1)
    if(ODIN_JUCE)
        target_compile_definitions(OdinReferenceCore PRIVATE ODIN_MAC=1)
    endif()
elseif(UNIX)
    target_compile_definitions(odin_reference PRIVATE ODIN_LINUX=1)
    if(ODIN_JUCE)
        target_compile_definitions(OdinReferenceCore PRIVATE ODIN_LINUX=1)
    endif()
else()
    target_compile_definitions(odin_reference PRIVATE ODIN_WIN=1)
    if(ODIN_JUCE)
        target_compile_definitions(OdinReferenceCore PRIVATE ODIN_WIN=1)
    endif()
endif()

# --- Banc des effets (Delay/Chorus/Flanger/Phaser/ZitaReverb): vraies unités Source/audio/FX ---
if(ODIN_JUCE AND NOT ODIN_AUDIO_MODULES_ONLY)
    juce_add_console_app(odin_fx_modules)
    set(ODIN_FX_AUDIO_DIR "${ODIN_UPSTREAM}/Source/audio")
    target_sources(odin_fx_modules PRIVATE fx_modules.cpp
        "${ODIN_FX_AUDIO_DIR}/FX/Delay.cpp" "${ODIN_FX_AUDIO_DIR}/FX/Chorus.cpp"
        "${ODIN_FX_AUDIO_DIR}/FX/Flanger.cpp" "${ODIN_FX_AUDIO_DIR}/FX/Phaser.cpp"
        "${ODIN_FX_AUDIO_DIR}/FX/ZitaReverb.cpp"
        "${ODIN_FX_AUDIO_DIR}/Filters/CombFilter.cpp" "${ODIN_FX_AUDIO_DIR}/Filters/BiquadAllpass.cpp"
        "${ODIN_FX_AUDIO_DIR}/Filters/BiquadFilter.cpp" "${ODIN_FX_AUDIO_DIR}/Filters/BiquadEQ.cpp"
        "${ODIN_FX_AUDIO_DIR}/Filters/VAOnePoleFilter.cpp" "${ODIN_FX_AUDIO_DIR}/Filters/OdinFilterBase.cpp")
    target_compile_features(odin_fx_modules PRIVATE cxx_std_17)
    target_link_libraries(odin_fx_modules PRIVATE OdinModuleAssets juce::juce_audio_processors juce::juce_dsp)
    target_include_directories(odin_fx_modules PRIVATE "${ODIN_UPSTREAM}/Source" "${CMAKE_BINARY_DIR}/include")
    target_compile_definitions(odin_fx_modules PRIVATE
        JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_MODAL_LOOPS_PERMITTED=1
        CMAKE_SOURCE_DIRECTORY="${ODIN_UPSTREAM}")
    juce_generate_juce_header(odin_fx_modules)
    # Les fusions FMA d'Apple clang arm64 (a - b*c en une seule opération arrondie) ne sont pas portables:
    # Rust n'en fait jamais. "off" = arrondi séparé comme IEEE strict;
    # "on" = contraction par défaut du compilateur (fmadd arm64) (utile pour mesurer cet effet).
    set(ODIN_FX_FP_CONTRACT "off" CACHE STRING "-ffp-contract pour odin_fx_modules (off|on|fast)")
    target_compile_options(odin_fx_modules PRIVATE -ffp-contract=${ODIN_FX_FP_CONTRACT})
endif()

# --- Banc oscillateurs/LFO: vraies unités Source/audio/Oscillators (Wavetable2D, Multi, Vector, Chiptune, FM, PM, LFO) ---
if(ODIN_JUCE AND NOT ODIN_AUDIO_MODULES_ONLY)
    juce_add_console_app(odin_osc_modules)
    set(ODIN_OSC_AUDIO_DIR "${ODIN_UPSTREAM}/Source/audio")
    file(GLOB ODIN_OSC_SOURCES CONFIGURE_DEPENDS
        "${ODIN_OSC_AUDIO_DIR}/Oscillators/*.cpp"
        "${ODIN_OSC_AUDIO_DIR}/Oscillators/Wavetables/Tables/*.cpp")
    target_sources(odin_osc_modules PRIVATE osc_modules.cpp ${ODIN_OSC_SOURCES}
        "${ODIN_OSC_AUDIO_DIR}/Filters/VAOnePoleFilter.cpp" "${ODIN_OSC_AUDIO_DIR}/Filters/OdinFilterBase.cpp")
    target_compile_features(odin_osc_modules PRIVATE cxx_std_17)
    target_link_libraries(odin_osc_modules PRIVATE OdinModuleAssets juce::juce_audio_processors juce::juce_dsp)
    target_include_directories(odin_osc_modules PRIVATE "${ODIN_UPSTREAM}/Source" "${CMAKE_BINARY_DIR}/include")
    target_compile_definitions(odin_osc_modules PRIVATE
        JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_MODAL_LOOPS_PERMITTED=1
        CMAKE_SOURCE_DIRECTORY="${ODIN_UPSTREAM}")
    juce_generate_juce_header(odin_osc_modules)
endif()
