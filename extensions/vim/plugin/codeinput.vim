if exists('g:loaded_codeinput')
    finish
endif
let g:loaded_codeinput = 1

if !has('job') || !has('channel')
    echoerr 'CodeInput requires Vim with +job and +channel features'
    finish
endif

command! CodeInputInfo call codeinput#show_info()
command! CodeInputRefresh call codeinput#refresh()
command! CodeInputStop call codeinput#stop()

augroup CodeInput
    autocmd!
    autocmd VimLeavePre * call codeinput#stop()
augroup END

call codeinput#init()
