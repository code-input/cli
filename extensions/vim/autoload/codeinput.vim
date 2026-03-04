let s:job = v:null
let s:channel = v:null
let s:request_id = 0
let s:pending_requests = {}
let s:initialized = v:false
let s:root_uri = ''

function! codeinput#init() abort
    let s:root_uri = 'file://' . getcwd()
    let binary = get(g:, 'codeinput_binary', 'ci')
    
    if !executable(binary)
        echoerr 'CodeInput: binary not found: ' . binary
        return
    endif
    
    let cmd = [binary, 'lsp']
    let s:job = job_start(cmd, {
        \ 'out_cb': function('s:handle_message'),
        \ 'err_cb': function('s:handle_error'),
        \ 'out_mode': 'raw',
        \ 'noblock': 1,
    \ })
    
    if job_status(s:job) !=# 'run'
        echoerr 'CodeInput: failed to start LSP server'
        let s:job = v:null
        return
    endif
    
    let s:channel = job_getchannel(s:job)
    call s:send_request('initialize', {
        \ 'processId': getpid(),
        \ 'rootUri': s:root_uri,
        \ 'capabilities': {},
    \ })
endfunction

function! codeinput#stop() abort
    if s:job isnot v:null
        call job_stop(s:job)
        let s:job = v:null
        let s:channel = v:null
        let s:initialized = v:false
    endif
endfunction

function! codeinput#show_info() abort
    if !s:initialized
        echo 'CodeInput: not initialized'
        return
    endif
    
    let uri = 'file://' . expand('%:p')
    call s:send_request('textDocument/hover', {
        \ 'textDocument': {'uri': uri},
        \ 'position': {'line': line('.') - 1, 'character': col('.') - 1},
    \ }, function('s:show_hover'))
endfunction

function! codeinput#refresh() abort
    if !s:initialized
        echo 'CodeInput: not initialized'
        return
    endif
    
    call s:send_notification('workspace/executeCommand', {
        \ 'command': 'codeinput.refresh',
    \ })
    echo 'CodeInput: cache refreshed'
endfunction

function! s:send_request(method, params, ...) abort
    let s:request_id += 1
    let request = {
        \ 'jsonrpc': '2.0',
        \ 'id': s:request_id,
        \ 'method': a:method,
        \ 'params': a:params,
    \ }
    
    if a:0 > 0
        let s:pending_requests[s:request_id] = a:1
    endif
    
    call s:send_json(request)
endfunction

function! s:send_notification(method, params) abort
    let notification = {
        \ 'jsonrpc': '2.0',
        \ 'method': a:method,
        \ 'params': a:params,
    \ }
    call s:send_json(notification)
endfunction

function! s:send_json(data) abort
    if s:channel is v:null
        return
    endif
    
    let json = json_encode(a:data)
    let message = "Content-Length: " . strlen(json) . "\r\n\r\n" . json
    call ch_sendraw(s:channel, message)
endfunction

let s:buffer = ''

function! s:handle_message(channel, data) abort
    let s:buffer .= a:data
    
    while v:true
        let header_end = stridx(s:buffer, "\r\n\r\n")
        if header_end == -1
            break
        endif
        
        let header = s:buffer[:header_end - 1]
        let content_length = s:parse_content_length(header)
        if content_length == -1
            let s:buffer = s:buffer[header_end + 4:]
            continue
        endif
        
        let body_start = header_end + 4
        let body_end = body_start + content_length
        if strlen(s:buffer) < body_end
            break
        endif
        
        let body = s:buffer[body_start : body_end - 1]
        let s:buffer = s:buffer[body_end :]
        
        try
            let response = json_decode(body)
            call s:handle_response(response)
        catch
            echoerr 'CodeInput: failed to parse LSP message'
        endtry
    endwhile
endfunction

function! s:parse_content_length(header) abort
    let lines = split(a:header, "\r\n")
    for line in lines
        if line =~? '^Content-Length:'
            return str2nr(matchstr(line, '\d\+'))
        endif
    endfor
    return -1
endfunction

function! s:handle_response(response) abort
    if has_key(a:response, 'method') && a:response.method ==# 'initialize'
        let s:initialized = v:true
        call s:send_notification('initialized', {})
        return
    endif
    
    if has_key(a:response, 'id') && has_key(s:pending_requests, a:response.id)
        let Callback = s:pending_requests[a:response.id]
        unlet s:pending_requests[a:response.id]
        if has_key(a:response, 'result')
            call Callback(a:response.result)
        endif
    endif
endfunction

function! s:handle_error(channel, data) abort
    " Ignore progress messages and non-error output
    if a:data =~? 'error\|fail' && a:data !~? 'completed successfully'
        echoerr 'CodeInput LSP: ' . a:data
    endif
endfunction

function! s:show_hover(result) abort
    if type(a:result) == type(v:null)
        echo 'CodeInput: no ownership info'
        return
    endif
    
    if has_key(a:result, 'contents')
        let contents = a:result.contents
        if type(contents) == type('')
            echo contents
        elseif type(contents) == type([])
            echo join(contents, "\n")
        endif
    endif
endfunction
