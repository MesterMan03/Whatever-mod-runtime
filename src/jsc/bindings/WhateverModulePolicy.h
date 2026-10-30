#pragma once

#include "BunString.h"
#include <JavaScriptCore/Error.h>
#include <JavaScriptCore/JSGlobalObject.h>
#include <JavaScriptCore/ExceptionScope.h>
#include <wtf/URL.h>

extern "C" bool Bun__whateverIsLocked();
extern "C" bool Bun__whateverLock(const BunString*);
extern "C" bool Bun__whateverAllowsSpecifier(const BunString*);
extern "C" bool Bun__whateverAllowsResolved(const BunString*);

namespace Bun {

inline bool checkWhateverModule(JSC::JSGlobalObject* global, const WTF::String& name, bool resolved = false)
{
    if (!Bun__whateverIsLocked())
        return true;
    auto path = name;
    if (path.startsWith("file://"_s)) {
        WTF::URL url(path);
        if (url.isValid() && url.host().isEmpty())
            path = url.fileSystemPath();
    }
    auto specifier = Bun::toString(path);
    if (resolved ? Bun__whateverAllowsResolved(&specifier) : Bun__whateverAllowsSpecifier(&specifier))
        return true;
    auto scope = DECLARE_THROW_SCOPE(global->vm());
    JSC::throwTypeError(global, scope, makeString("Whatever module policy denied: "_s, name));
    return false;
}

} // namespace Bun